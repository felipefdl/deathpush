use crate::error::{Error, Result};
use crate::types::{DiffHunk, DiffLine};

pub fn parse_unified_diff(diff_output: &str) -> Vec<DiffHunk> {
  let mut hunks = Vec::new();
  let lines: Vec<&str> = diff_output.lines().collect();
  let mut i = 0;

  while i < lines.len() {
    if lines[i].starts_with("@@") {
      if let Some(hunk) = parse_hunk_header(lines[i]) {
        let mut hunk = hunk;
        let mut old_line = hunk.old_start;
        let mut new_line = hunk.new_start;
        i += 1;

        while i < lines.len() && !lines[i].starts_with("@@") && !lines[i].starts_with("diff --git") {
          let line = lines[i];
          if let Some(content) = line.strip_prefix('+') {
            hunk.lines.push(DiffLine {
              content: content.to_string(),
              line_type: "add".to_string(),
              old_line_number: None,
              new_line_number: Some(new_line),
            });
            new_line += 1;
          } else if let Some(content) = line.strip_prefix('-') {
            hunk.lines.push(DiffLine {
              content: content.to_string(),
              line_type: "remove".to_string(),
              old_line_number: Some(old_line),
              new_line_number: None,
            });
            old_line += 1;
          } else if line.starts_with(' ') || line.is_empty() {
            let content = if line.is_empty() { "" } else { &line[1..] };
            hunk.lines.push(DiffLine {
              content: content.to_string(),
              line_type: "context".to_string(),
              old_line_number: Some(old_line),
              new_line_number: Some(new_line),
            });
            old_line += 1;
            new_line += 1;
          } else {
            // No newline at end of file marker or other non-diff line
            i += 1;
            continue;
          }
          i += 1;
        }

        hunks.push(hunk);
        continue;
      }
    }
    i += 1;
  }

  hunks
}

fn parse_hunk_header(line: &str) -> Option<DiffHunk> {
  // Format: @@ -old_start,old_lines +new_start,new_lines @@ optional text
  let line = line.strip_prefix("@@ ")?;
  let end = line.find(" @@")?;
  let range_part = &line[..end];
  let header_text = if end + 3 < line.len() {
    line[end + 3..].trim().to_string()
  } else {
    String::new()
  };

  let parts: Vec<&str> = range_part.split(' ').collect();
  if parts.len() != 2 {
    return None;
  }

  let old_range = parts[0].strip_prefix('-')?;
  let new_range = parts[1].strip_prefix('+')?;

  let (old_start, old_lines) = parse_range(old_range)?;
  let (new_start, new_lines) = parse_range(new_range)?;

  Some(DiffHunk {
    header: format!("@@ -{} +{} @@ {}", old_range, new_range, header_text)
      .trim_end()
      .to_string(),
    old_start,
    old_lines,
    new_start,
    new_lines,
    lines: Vec::new(),
  })
}

fn parse_range(range: &str) -> Option<(usize, usize)> {
  if let Some((start, lines)) = range.split_once(',') {
    Some((start.parse().ok()?, lines.parse().ok()?))
  } else {
    Some((range.parse().ok()?, 1))
  }
}

pub fn generate_hunk_patch(path: &str, diff_output: &str, hunk_index: usize) -> Result<String> {
  let lines: Vec<&str> = diff_output.lines().collect();

  // Extract file header lines (everything before the first @@)
  let mut file_header: Vec<String> = Vec::new();
  for line in &lines {
    if line.starts_with("@@") {
      break;
    }
    file_header.push(line.to_string());
  }

  // Ensure we have proper diff --git header
  let has_diff_header = file_header.iter().any(|l| l.starts_with("diff --git"));
  if !has_diff_header {
    file_header.insert(0, format!("diff --git a/{path} b/{path}"));
    file_header.push(format!("--- a/{path}"));
    file_header.push(format!("+++ b/{path}"));
  }

  // Find the target hunk
  let hunks = parse_unified_diff(diff_output);
  let hunk = hunks
    .get(hunk_index)
    .ok_or_else(|| Error::Other(format!("Hunk index {} out of range", hunk_index)))?;

  // Build the patch
  let mut patch = String::new();
  for line in &file_header {
    patch.push_str(line);
    patch.push('\n');
  }
  patch.push_str(&hunk.header);
  patch.push('\n');

  for diff_line in &hunk.lines {
    match diff_line.line_type.as_str() {
      "add" => {
        patch.push('+');
        patch.push_str(&diff_line.content);
        patch.push('\n');
      }
      "remove" => {
        patch.push('-');
        patch.push_str(&diff_line.content);
        patch.push('\n');
      }
      _ => {
        patch.push(' ');
        patch.push_str(&diff_line.content);
        patch.push('\n');
      }
    }
  }

  Ok(patch)
}

pub fn generate_lines_patch(
  path: &str,
  diff_output: &str,
  hunk_index: usize,
  line_start: usize,
  line_end: usize,
) -> Result<String> {
  let lines: Vec<&str> = diff_output.lines().collect();

  // Extract file header lines (everything before the first @@)
  let mut file_header: Vec<String> = Vec::new();
  for line in &lines {
    if line.starts_with("@@") {
      break;
    }
    file_header.push(line.to_string());
  }

  let has_diff_header = file_header.iter().any(|l| l.starts_with("diff --git"));
  if !has_diff_header {
    file_header.insert(0, format!("diff --git a/{path} b/{path}"));
    file_header.push(format!("--- a/{path}"));
    file_header.push(format!("+++ b/{path}"));
  }

  let hunks = parse_unified_diff(diff_output);
  let hunk = hunks
    .get(hunk_index)
    .ok_or_else(|| Error::Other(format!("Hunk index {} out of range", hunk_index)))?;

  // Build a partial patch with only the selected lines
  let mut old_count: usize = 0;
  let mut new_count: usize = 0;
  let mut patch_lines = Vec::new();

  for (i, diff_line) in hunk.lines.iter().enumerate() {
    let in_range = i >= line_start && i <= line_end;
    match diff_line.line_type.as_str() {
      "add" => {
        if in_range {
          patch_lines.push(format!("+{}", diff_line.content));
          new_count += 1;
        }
        // Out-of-range additions are simply omitted
      }
      "remove" => {
        if in_range {
          patch_lines.push(format!("-{}", diff_line.content));
          old_count += 1;
        } else {
          // Out-of-range removals become context
          patch_lines.push(format!(" {}", diff_line.content));
          old_count += 1;
          new_count += 1;
        }
      }
      _ => {
        patch_lines.push(format!(" {}", diff_line.content));
        old_count += 1;
        new_count += 1;
      }
    }
  }

  let mut patch = String::new();
  for line in &file_header {
    patch.push_str(line);
    patch.push('\n');
  }
  patch.push_str(&format!(
    "@@ -{},{} +{},{} @@\n",
    hunk.old_start, old_count, hunk.new_start, new_count
  ));
  for line in &patch_lines {
    patch.push_str(line);
    patch.push('\n');
  }

  Ok(patch)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_unified_diff() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@
 fn main() {
-    println!(\"hello\");
+    println!(\"hello world\");
+    println!(\"goodbye\");
 }
";

    let hunks = parse_unified_diff(diff);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].old_start, 1);
    assert_eq!(hunks[0].old_lines, 4);
    assert_eq!(hunks[0].new_start, 1);
    assert_eq!(hunks[0].new_lines, 5);
    assert_eq!(hunks[0].lines.len(), 5);
    assert_eq!(hunks[0].lines[1].line_type, "remove");
    assert_eq!(hunks[0].lines[2].line_type, "add");
  }

  #[test]
  fn test_generate_hunk_patch() {
    let diff = "\
diff --git a/file.txt b/file.txt
index abc..def 100644
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line1
-line2
+line2_modified
 line3
";

    let patch = generate_hunk_patch("file.txt", diff, 0).unwrap();
    assert!(patch.contains("diff --git"));
    assert!(patch.contains("@@ -1,3 +1,3 @@"));
    assert!(patch.contains("-line2\n"));
    assert!(patch.contains("+line2_modified\n"));
  }

  #[test]
  fn test_multi_hunk_diff() {
    let diff = "\
diff --git a/file.rs b/file.rs
index abc..def 100644
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,4 @@
 line1
+inserted
 line2
 line3
@@ -10,3 +11,4 @@
 line10
+another
 line11
 line12
";

    let hunks = parse_unified_diff(diff);
    assert_eq!(hunks.len(), 2);
    assert_eq!(hunks[0].old_start, 1);
    assert_eq!(hunks[0].old_lines, 3);
    assert_eq!(hunks[0].new_start, 1);
    assert_eq!(hunks[0].new_lines, 4);
    assert_eq!(hunks[1].old_start, 10);
    assert_eq!(hunks[1].old_lines, 3);
    assert_eq!(hunks[1].new_start, 11);
    assert_eq!(hunks[1].new_lines, 4);
  }

  #[test]
  fn test_single_number_range() {
    let diff = "\
@@ -10 +10 @@
-old
+new
";

    let hunks = parse_unified_diff(diff);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].old_start, 10);
    assert_eq!(hunks[0].old_lines, 1);
    assert_eq!(hunks[0].new_start, 10);
    assert_eq!(hunks[0].new_lines, 1);
  }

  #[test]
  fn test_header_with_function_context() {
    let diff = "\
@@ -1,4 +1,5 @@ fn main()
 line1
+added
 line2
";

    let hunks = parse_unified_diff(diff);
    assert_eq!(hunks.len(), 1);
    assert!(hunks[0].header.contains("fn main()"));
  }

  #[test]
  fn test_generate_hunk_patch_second_hunk() {
    let diff = "\
diff --git a/file.rs b/file.rs
index abc..def 100644
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,3 @@
 line1
-old1
+new1
 line3
@@ -10,3 +10,3 @@
 line10
-old2
+new2
 line12
";

    let patch = generate_hunk_patch("file.rs", diff, 1).unwrap();
    assert!(patch.contains("@@ -10,3 +10,3 @@"));
    assert!(patch.contains("-old2\n"));
    assert!(patch.contains("+new2\n"));
    assert!(!patch.contains("-old1"));
  }

  #[test]
  fn test_generate_hunk_patch_synthesizes_header() {
    let diff = "\
@@ -1,3 +1,3 @@
 line1
-old
+new
 line3
";

    let patch = generate_hunk_patch("test.rs", diff, 0).unwrap();
    assert!(patch.contains("diff --git a/test.rs b/test.rs"));
    assert!(patch.contains("--- a/test.rs"));
    assert!(patch.contains("+++ b/test.rs"));
  }

  #[test]
  fn test_generate_hunk_patch_out_of_range() {
    let diff = "\
diff --git a/file.rs b/file.rs
--- a/file.rs
+++ b/file.rs
@@ -1,3 +1,3 @@
 line1
-old
+new
 line3
";

    let result = generate_hunk_patch("file.rs", diff, 5);
    assert!(result.is_err());
  }

  // --- generate_lines_patch tests ---

  const LINES_PATCH_DIFF: &str = "\
diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -1,5 +1,6 @@
 context1
-removed1
-removed2
+added1
+added2
+added3
 context2
";

  #[test]
  fn test_generate_lines_patch_select_all() {
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 0, 6).unwrap();
    assert!(patch.contains("-removed1\n"));
    assert!(patch.contains("-removed2\n"));
    assert!(patch.contains("+added1\n"));
    assert!(patch.contains("+added2\n"));
    assert!(patch.contains("+added3\n"));
    assert!(patch.contains(" context1\n"));
    assert!(patch.contains(" context2\n"));
  }

  #[test]
  fn test_generate_lines_patch_select_single_add() {
    // Select only line 3 (added1); other adds omitted, removes become context
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 3, 3).unwrap();
    assert!(patch.contains("+added1\n"));
    assert!(!patch.contains("+added2"));
    assert!(!patch.contains("+added3"));
    // Out-of-range removes become context
    assert!(patch.contains(" removed1\n"));
    assert!(patch.contains(" removed2\n"));
  }

  #[test]
  fn test_generate_lines_patch_select_single_remove() {
    // Select only line 1 (removed1); other removes become context, adds omitted
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 1, 1).unwrap();
    assert!(patch.contains("-removed1\n"));
    assert!(patch.contains(" removed2\n")); // out-of-range remove -> context
    assert!(!patch.contains("+added1"));
    assert!(!patch.contains("+added2"));
    assert!(!patch.contains("+added3"));
  }

  #[test]
  fn test_generate_lines_patch_out_of_range_adds_omitted() {
    // Select only removes (lines 1-2); all adds should be omitted entirely
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 1, 2).unwrap();
    assert!(patch.contains("-removed1\n"));
    assert!(patch.contains("-removed2\n"));
    assert!(!patch.contains("+added1"));
    assert!(!patch.contains("+added2"));
    assert!(!patch.contains("+added3"));
  }

  #[test]
  fn test_generate_lines_patch_out_of_range_removes_become_context() {
    // Select only adds (lines 3-5); removes should become context (space prefix)
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 3, 5).unwrap();
    assert!(patch.contains(" removed1\n"));
    assert!(patch.contains(" removed2\n"));
    assert!(!patch.contains("-removed1"));
    assert!(!patch.contains("-removed2"));
  }

  #[test]
  fn test_generate_lines_patch_context_lines_unchanged() {
    // Select only the middle (lines 2-4); context lines still pass through
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 2, 4).unwrap();
    assert!(patch.contains(" context1\n"));
    assert!(patch.contains(" context2\n"));
  }

  #[test]
  fn test_generate_lines_patch_header_counts() {
    // Select single add (line 3): old_count = 2 context + 2 removes-as-context = 4, new_count = 4 + 1 add = 5
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 3, 3).unwrap();
    assert!(patch.contains("@@ -1,4 +1,5 @@"));

    // Select single remove (line 1): old_count = 2 context + 1 remove + 1 remove-as-context = 4, new_count = 2 + 1 = 3
    let patch = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 0, 1, 1).unwrap();
    assert!(patch.contains("@@ -1,4 +1,3 @@"));
  }

  #[test]
  fn test_generate_lines_patch_synthesizes_header() {
    let diff_no_header = "\
@@ -1,5 +1,6 @@
 context1
-removed1
-removed2
+added1
+added2
+added3
 context2
";
    let patch = generate_lines_patch("my/file.txt", diff_no_header, 0, 0, 6).unwrap();
    assert!(patch.contains("diff --git a/my/file.txt b/my/file.txt"));
    assert!(patch.contains("--- a/my/file.txt"));
    assert!(patch.contains("+++ b/my/file.txt"));
  }

  #[test]
  fn test_generate_lines_patch_invalid_hunk_index() {
    let result = generate_lines_patch("file.txt", LINES_PATCH_DIFF, 5, 0, 6);
    assert!(result.is_err());
  }
}
