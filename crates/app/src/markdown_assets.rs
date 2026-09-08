//! Local image support for the markdown preview.
//!
//! `gpui-base` hands every markdown image URL to GPUI verbatim as a `Resource::Uri`, and GPUI
//! loads a `Resource::Uri` exclusively through `cx.http_client()`. A relative `![](docs/shot.png)`
//! therefore reaches the http client with no idea which document wrote it, and GPUI's image cache
//! is keyed by that same string, so two READMEs asking for `docs/shot.png` would share one entry.
//!
//! So the preview rewrites its own local image URLs to absolute `file://` URLs before the document
//! is parsed, grants exactly those files, and this client serves nothing else. Remote URLs go to
//! the network untouched.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use gpui_kit::http_client::http::{HeaderValue, Request, Response, StatusCode};
use gpui_kit::http_client::{AsyncBody, HttpClient, Result as HttpResult, Url};
use gpui_kit::*;
use markdown::mdast::Node;
use percent_encoding::percent_decode_str;

use crate::repo::image_load::bounded_bytes;

/// The files open markdown previews may read, one set per preview.
#[derive(Default)]
pub struct AssetGrants {
  granted: Mutex<HashMap<u64, HashSet<PathBuf>>>,
}

impl AssetGrants {
  /// A preview id no other preview will reuse.
  pub fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
  }

  pub fn grant(&self, preview: u64, files: HashSet<PathBuf>) {
    let mut granted = self.lock();
    if files.is_empty() {
      granted.remove(&preview);
    } else {
      granted.insert(preview, files);
    }
  }

  pub fn revoke(&self, preview: u64) {
    self.lock().remove(&preview);
  }

  fn allows(&self, file: &Path) -> bool {
    self.lock().values().any(|files| files.contains(file))
  }

  #[cfg(test)]
  pub(crate) fn allows_file(&self, file: &Path) -> bool {
    self.allows(file)
  }

  fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, HashSet<PathBuf>>> {
    self.granted.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
  }
}

/// App-wide handle on the grant table, so a preview can register its own files.
pub struct MarkdownAssets(pub Arc<AssetGrants>);

impl Global for MarkdownAssets {}

pub fn grants(cx: &App) -> Option<Arc<AssetGrants>> {
  cx.try_global::<MarkdownAssets>().map(|assets| assets.0.clone())
}

/// A markdown source whose local image URLs have been rewritten, plus the files that unlocks.
pub struct Rewritten {
  pub source: String,
  pub files: HashSet<PathBuf>,
}

/// Rewrite the local image URLs in `source` to absolute `file://` URLs. `document` is the file
/// being previewed (it may not exist yet) and `boundary` is the repository root: an image that
/// resolves outside the repository is left exactly as written and never granted.
pub fn rewrite_images(source: &str, document: &Path, boundary: &Path) -> Rewritten {
  let directory = document.parent().unwrap_or(boundary);
  let Ok(tree) = markdown::to_mdast(source, &markdown::ParseOptions::gfm()) else {
    return Rewritten {
      source: source.to_string(),
      files: HashSet::new(),
    };
  };

  let mut images = Vec::new();
  let mut definitions = Vec::new();
  let mut referenced = HashSet::new();
  collect(&tree, &mut images, &mut definitions, &mut referenced);
  // A reference-style image resolves through its definition, so that line carries the URL.
  images.extend(
    definitions
      .into_iter()
      .filter(|(identifier, _, _)| referenced.contains(identifier))
      .map(|(_, span, url)| (span, url)),
  );

  let mut files = HashSet::new();
  let mut edits = Vec::new();
  for (span, url) in images {
    let Some((minted, file)) = mint(&url, directory, boundary) else {
      continue;
    };
    let Some(range) = destination_range(source, span, &url) else {
      continue;
    };
    files.insert(file);
    edits.push((range, minted));
  }

  edits.sort_by_key(|((start, _), _)| *start);
  let mut rewritten = String::with_capacity(source.len());
  let mut cursor = 0;
  for ((start, end), minted) in edits {
    if start < cursor {
      continue;
    }
    rewritten.push_str(&source[cursor..start]);
    rewritten.push_str(&minted);
    cursor = end;
  }
  rewritten.push_str(&source[cursor..]);
  Rewritten {
    source: rewritten,
    files,
  }
}

type Span = (usize, usize);

fn collect(
  node: &Node,
  images: &mut Vec<(Span, String)>,
  definitions: &mut Vec<(String, Span, String)>,
  referenced: &mut HashSet<String>,
) {
  match node {
    Node::Image(image) => {
      if let Some(span) = span_of(image.position.as_ref()) {
        images.push((span, image.url.clone()));
      }
    }
    Node::ImageReference(reference) => {
      referenced.insert(reference.identifier.clone());
    }
    Node::Definition(definition) => {
      if let Some(span) = span_of(definition.position.as_ref()) {
        definitions.push((definition.identifier.clone(), span, definition.url.clone()));
      }
    }
    _ => {}
  }
  if let Some(children) = node.children() {
    for child in children {
      collect(child, images, definitions, referenced);
    }
  }
}

fn span_of(position: Option<&markdown::unist::Position>) -> Option<Span> {
  position.map(|position| (position.start.offset, position.end.offset))
}

/// Where the destination sits inside an image span or a link definition line. Anchored on the
/// syntax that introduces it, so a title repeating the URL cannot be rewritten by mistake.
fn destination_range(source: &str, (start, end): Span, url: &str) -> Option<Span> {
  let span = source.get(start..end)?;
  let after = span
    .find("](")
    .map(|at| at + 2)
    .or_else(|| span.find("]:").map(|at| at + 2))?;
  let at = span.get(after..)?.find(url)? + after;
  Some((start + at, start + at + url.len()))
}

/// `http::Uri` refuses an empty authority, and GPUI parses the URL into one before it ever
/// reaches this client, so a minted URL has to name a host.
const LOCAL_HOST: &str = "localhost";

/// The URL for a local image, or `None` for anything remote, missing, or outside the repository.
/// The modification time rides along so an edited image gets a new cache key.
fn mint(url: &str, directory: &Path, boundary: &Path) -> Option<(String, PathBuf)> {
  let file = resolve(url, directory, boundary)?;
  let modified = std::fs::metadata(&file)
    .and_then(|metadata| metadata.modified())
    .ok()
    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|since| since.as_nanos())
    .unwrap_or(0);
  let mut url = Url::from_file_path(&file).ok()?;
  if !url.has_host() {
    url.set_host(Some(LOCAL_HOST)).ok()?;
  }
  url.set_query(Some(&format!("v={modified}")));
  Some((url.into(), file))
}

/// Resolve a markdown image reference against the document's own directory. `None` when the
/// reference is remote, escapes the repository, or is not a readable file.
pub(crate) fn resolve(url: &str, directory: &Path, boundary: &Path) -> Option<PathBuf> {
  let raw = match scheme(url) {
    Some("file") => url.strip_prefix("file://").unwrap_or(url.strip_prefix("file:")?),
    // Remote images, data URIs and anything else stay someone else's problem.
    Some(_) => return None,
    None => url,
  };
  let raw = raw.split(['?', '#']).next()?;
  if raw.is_empty() {
    return None;
  }
  let decoded = percent_decode_str(raw).decode_utf8_lossy().into_owned();
  let candidate = if Path::new(&decoded).is_absolute() {
    PathBuf::from(decoded)
  } else {
    directory.join(decoded)
  };
  let canon_boundary = boundary.canonicalize().ok()?;
  let canon = candidate.canonicalize().ok()?;
  // A document must not read outside its repository just because it asked politely.
  if !canon.starts_with(&canon_boundary) || !canon.is_file() {
    return None;
  }
  Some(canon)
}

fn scheme(url: &str) -> Option<&str> {
  let end = url.find(':')?;
  let scheme = &url[..end];
  let mut chars = scheme.chars();
  let first = chars.next()?;
  (first.is_ascii_alphabetic() && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then_some(scheme)
}

/// Links a preview may hand to the desktop. A document is untrusted input; `file:` and friends
/// are not opened just because someone wrote them down.
pub fn is_safe_link(url: &str) -> bool {
  matches!(scheme(url), Some("http") | Some("https") | Some("mailto"))
}

pub struct LocalAssetHttpClient {
  inner: Arc<dyn HttpClient>,
  grants: Arc<AssetGrants>,
}

impl LocalAssetHttpClient {
  pub fn new(inner: Arc<dyn HttpClient>) -> (Arc<Self>, Arc<AssetGrants>) {
    let grants = Arc::new(AssetGrants::default());
    (
      Arc::new(Self {
        inner,
        grants: grants.clone(),
      }),
      grants,
    )
  }
}

impl HttpClient for LocalAssetHttpClient {
  fn user_agent(&self) -> Option<&HeaderValue> {
    self.inner.user_agent()
  }

  fn proxy(&self) -> Option<&Url> {
    self.inner.proxy()
  }

  fn send(&self, req: Request<AsyncBody>) -> futures::future::BoxFuture<'static, HttpResult<Response<AsyncBody>>> {
    if req.uri().scheme_str() != Some("file") {
      return self.inner.send(req);
    }
    let uri = req.uri().to_string();
    let grants = self.grants.clone();
    Box::pin(async move {
      let Some(file) = granted_path(&uri, &grants) else {
        tracing::debug!(uri, "markdown asset is not granted to any preview");
        return Ok(not_found());
      };
      match std::fs::read(&file) {
        // GPUI uploads whatever it decodes straight to the texture atlas, so the bytes it sees
        // have to be inside the atlas limit already.
        Ok(bytes) => Ok(
          Response::builder()
            .status(StatusCode::OK)
            .body(AsyncBody::from(bounded_bytes(&file.to_string_lossy(), bytes)))?,
        ),
        Err(err) => {
          tracing::debug!(?file, %err, "markdown asset could not be read");
          Ok(not_found())
        }
      }
    })
  }
}

fn granted_path(uri: &str, grants: &AssetGrants) -> Option<PathBuf> {
  let url = Url::parse(uri).ok()?;
  if url.scheme() != "file" {
    return None;
  }
  let file = url.to_file_path().ok()?.canonicalize().ok()?;
  grants.allows(&file).then_some(file)
}

fn not_found() -> Response<AsyncBody> {
  Response::builder()
    .status(StatusCode::NOT_FOUND)
    .body(AsyncBody::default())
    .expect("static response builds")
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  struct Repo {
    dir: tempfile::TempDir,
  }

  impl Repo {
    /// A repository with `docs/shot png.png` and a README beside it.
    fn new() -> Self {
      let dir = tempfile::tempdir().expect("tempdir");
      std::fs::create_dir_all(dir.path().join("docs")).expect("docs dir");
      std::fs::write(dir.path().join("docs/shot png.png"), b"x").expect("asset");
      std::fs::write(dir.path().join("secret.png"), b"x").expect("asset");
      Self { dir }
    }

    fn root(&self) -> &Path {
      self.dir.path()
    }
  }

  #[test]
  fn local_images_become_granted_file_urls() {
    let repo = Repo::new();
    let readme = repo.root().join("README.md");
    let rewritten = rewrite_images(
      "![shot](docs/shot%20png.png)\n\n![remote](https://example.com/a.png)\n",
      &readme,
      repo.root(),
    );

    assert_eq!(rewritten.files.len(), 1, "only the local image is granted");
    assert!(
      rewritten.source.contains("(file://") && rewritten.source.contains("?v="),
      "local image must be rewritten with a cache key: {}",
      rewritten.source
    );
    assert!(
      rewritten.source.contains("https://example.com/a.png"),
      "a remote image must survive untouched"
    );
  }

  #[test]
  fn a_minted_url_survives_the_http_request_gpui_builds() {
    let repo = Repo::new();
    let readme = repo.root().join("README.md");
    let rewritten = rewrite_images("![shot](docs/shot%20png.png)", &readme, repo.root());
    let url = rewritten
      .source
      .trim_start_matches("![shot](")
      .trim_end_matches(')')
      .to_string();

    // GPUI turns a document's image URL into exactly this request; an unparseable URL means no
    // picture at all, which is how `file:///path` failed.
    let request = Request::builder()
      .uri(url.as_str())
      .body(AsyncBody::default())
      .unwrap_or_else(|err| panic!("gpui cannot request {url}: {err}"));
    assert_eq!(request.uri().scheme_str(), Some("file"));

    let grants = AssetGrants::default();
    let preview = AssetGrants::next_id();
    grants.grant(preview, rewritten.files);
    assert!(
      granted_path(&request.uri().to_string(), &grants).is_some(),
      "the client must resolve the url it minted: {url}"
    );
    grants.revoke(preview);
    assert!(
      granted_path(&request.uri().to_string(), &grants).is_none(),
      "a closed preview keeps no access"
    );
  }

  struct NoNetwork;

  impl HttpClient for NoNetwork {
    fn user_agent(&self) -> Option<&HeaderValue> {
      None
    }

    fn proxy(&self) -> Option<&Url> {
      None
    }

    fn send(&self, req: Request<AsyncBody>) -> futures::future::BoxFuture<'static, HttpResult<Response<AsyncBody>>> {
      panic!("a local asset must never reach the network: {}", req.uri());
    }
  }

  fn fetch(client: &LocalAssetHttpClient, url: &str) -> (StatusCode, Vec<u8>) {
    use futures::AsyncReadExt as _;
    let request = Request::builder()
      .uri(url)
      .body(AsyncBody::default())
      .expect("gpui builds this request");
    futures::executor::block_on(async {
      let mut response = client.send(request).await.expect("the client answers");
      let status = response.status();
      let mut body = Vec::new();
      response.body_mut().read_to_end(&mut body).await.expect("body reads");
      (status, body)
    })
  }

  #[test]
  fn the_client_serves_a_granted_image_inside_the_atlas_limit() {
    let repo = Repo::new();
    let wide = image::RgbaImage::from_pixel(9000, 60, image::Rgba([9, 9, 9, 255]));
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(wide)
      .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
      .expect("encode png");
    std::fs::write(repo.root().join("wide #%.png"), &png).expect("asset");

    let readme = repo.root().join("README.md");
    let rewritten = rewrite_images("![wide](wide%20%23%25.png)", &readme, repo.root());
    let url = rewritten
      .source
      .trim_start_matches("![wide](")
      .trim_end_matches(')')
      .to_string();

    let (client, grants) = LocalAssetHttpClient::new(Arc::new(NoNetwork));
    let preview = AssetGrants::next_id();
    grants.grant(preview, rewritten.files);

    let (status, body) = fetch(&client, &url);
    assert_eq!(status, StatusCode::OK);
    let dimensions = image::ImageReader::new(std::io::Cursor::new(&body))
      .with_guessed_format()
      .expect("format")
      .into_dimensions()
      .expect("the served bytes must be an image");
    assert!(
      dimensions.0 <= crate::repo::image_load::MAX_IMAGE_EDGE,
      "served {dimensions:?}, which gpui would upload straight past the atlas limit"
    );

    // Anything the previews did not resolve is refused, however the document spells it.
    let outside = format!("file://{LOCAL_HOST}/etc/hosts");
    assert_eq!(fetch(&client, &outside).0, StatusCode::NOT_FOUND);
    grants.revoke(preview);
    assert_eq!(fetch(&client, &url).0, StatusCode::NOT_FOUND);
  }

  #[test]
  fn reference_style_images_are_rewritten_and_plain_links_are_not() {
    let repo = Repo::new();
    std::fs::write(repo.root().join("docs/spec.md"), "spec").expect("doc");
    let readme = repo.root().join("README.md");
    let rewritten = rewrite_images(
      "![shot][pic]\n\n[spec]: docs/spec.md\n[pic]: docs/shot%20png.png\n",
      &readme,
      repo.root(),
    );

    assert!(
      rewritten.source.contains("[pic]: file://"),
      "an image definition must be rewritten: {}",
      rewritten.source
    );
    assert!(
      rewritten.source.contains("[spec]: docs/spec.md"),
      "a link definition is not an image and stays put"
    );
  }

  #[test]
  fn code_blocks_are_left_alone() {
    let repo = Repo::new();
    let readme = repo.root().join("README.md");
    let source = "```md\n![shot](docs/shot%20png.png)\n```\n\n`![shot](docs/shot%20png.png)`\n";
    let rewritten = rewrite_images(source, &readme, repo.root());
    assert_eq!(rewritten.source, source, "a literal example is not a picture");
    assert!(rewritten.files.is_empty());
  }

  #[test]
  fn resolution_stays_inside_the_repository() {
    let repo = Repo::new();
    let docs = repo.root().join("docs");

    assert!(
      resolve("../secret.png", &docs, repo.root()).is_some(),
      "a relative path may walk up inside the repository"
    );
    assert!(resolve("shot%20png.png", &docs, repo.root()).is_some());
    assert!(resolve("./shot png.png?raw=true", &docs, repo.root()).is_some());
    assert!(resolve("shot png.png", &docs, docs.as_path()).is_some());
    assert!(
      resolve("../secret.png", &docs, docs.as_path()).is_none(),
      "the boundary is the boundary"
    );
    assert!(resolve("/etc/hosts", &docs, repo.root()).is_none());
    assert!(resolve("https://example.com/a.png", &docs, repo.root()).is_none());
    assert!(resolve("docs/missing.png", repo.root(), repo.root()).is_none());
  }

  #[test]
  fn only_web_links_are_handed_to_the_desktop() {
    assert!(is_safe_link("https://example.com"));
    assert!(is_safe_link("http://example.com"));
    assert!(is_safe_link("mailto:someone@example.com"));
    assert!(!is_safe_link("./docs/spec.md"), "a relative path is not a desktop url");
    assert!(!is_safe_link("file:///etc/passwd"));
    assert!(!is_safe_link("javascript:alert(1)"));
  }
}
