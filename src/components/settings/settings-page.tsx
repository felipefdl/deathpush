import { createSignal, createUniqueId, For, onSettled, Show, untrack } from "solid-js";
import { settingsStore } from "../../stores/settings-store";
import type {
  EditorSettings,
  FontWeight,
  GitSettings,
  ProjectsSettings,
  TerminalSettings,
  UISettings,
} from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { iconThemeStore } from "../../stores/icon-theme-store";
import { THEME_ENTRIES } from "../../lib/themes/theme-registry";
import { confirm } from "@tauri-apps/plugin-dialog";
import { getGitConfig, setGitConfig } from "../../lib/tauri-commands";
import { WorkspaceConfigModal } from "../shared/workspace-config-modal";
import { useStore } from "../../lib/use-store";
import { PLATFORM } from "../../lib/platform";
import "../../styles/settings.css";

export const SettingsPage = () => {
  const settings = useStore(settingsStore, (s) => s.settings);
  const { updateUI, updateEditor, updateTerminal, updateGit, updateProjects, resetToDefaults } =
    settingsStore.getState();

  const handleResetToDefaults = async () => {
    const confirmed = await confirm("Reset all settings to defaults? This cannot be undone.", {
      title: "Reset to Defaults",
      kind: "warning",
      okLabel: "Reset",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    resetToDefaults();
  };

  return (
    <div class="settings-page">
      <div class="settings-header">
        <span class="settings-title">Settings</span>
        <button class="settings-reset-btn" onClick={handleResetToDefaults}>
          Reset to Defaults
        </button>
      </div>
      <div class="settings-content">
        <AppearanceSection settings={settings().ui} onUpdate={updateUI} />
        <EditorSection settings={settings().editor} onUpdate={updateEditor} />
        <GitSection settings={settings().git} onUpdate={updateGit} />
        <ProjectsSection settings={settings().projects} onUpdate={updateProjects} />
        <TerminalSection
          settings={settings().terminal}
          onUpdate={updateTerminal}
          uiSettings={settings().ui}
          onUpdateUI={updateUI}
        />
      </div>
    </div>
  );
};

const AppearanceSection = (props: { settings: UISettings; onUpdate: (partial: Partial<UISettings>) => void }) => {
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const preferredDarkThemeId = useStore(themeStore, (s) => s.preferredDarkThemeId);
  const preferredLightThemeId = useStore(themeStore, (s) => s.preferredLightThemeId);
  const { setPreferredDarkTheme, setPreferredLightTheme } = themeStore.getState();
  const currentIconTheme = useStore(iconThemeStore, (s) => s.currentIconTheme);

  const darkThemes = THEME_ENTRIES.filter((t) => t.kind === "dark" || t.kind === "hc-dark");
  const lightThemes = THEME_ENTRIES.filter((t) => t.kind === "light" || t.kind === "hc-light");

  return (
    <div class="settings-section">
      <div class="settings-section-title">Appearance</div>
      <div class="settings-field">
        <span class="settings-label">Color Theme</span>
        <button
          class="settings-input settings-picker-btn"
          onClick={() => window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"))}
        >
          {currentTheme().label}
          <span class="settings-picker-hint">Cmd+K Cmd+T</span>
        </button>
      </div>
      <SelectField
        label="Preferred Dark Theme"
        value={preferredDarkThemeId()}
        options={darkThemes.map((t) => ({ value: t.id, label: t.label }))}
        onChange={setPreferredDarkTheme}
      />
      <SelectField
        label="Preferred Light Theme"
        value={preferredLightThemeId()}
        options={lightThemes.map((t) => ({ value: t.id, label: t.label }))}
        onChange={setPreferredLightTheme}
      />
      <div class="settings-field">
        <span class="settings-label">File Icon Theme</span>
        <button
          class="settings-input settings-picker-btn"
          onClick={() => window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"))}
        >
          {currentIconTheme().label}
          <span class="settings-picker-hint">Cmd+K Cmd+I</span>
        </button>
      </div>
      <SelectField
        label="Sidebar Position"
        value={props.settings.sidebarPosition}
        options={[
          { value: "left", label: "Left" },
          { value: "right", label: "Right" },
        ]}
        onChange={(v) => props.onUpdate({ sidebarPosition: v as UISettings["sidebarPosition"] })}
      />
      <TextField
        label="UI Font Family"
        value={props.settings.fontFamily}
        onChange={(v) => props.onUpdate({ fontFamily: v })}
      />
      <NumberField
        label="UI Font Size"
        value={props.settings.fontSize}
        onChange={(v) => props.onUpdate({ fontSize: v })}
        min={10}
        max={20}
      />
      <SelectField
        label="Zoom"
        value={String(props.settings.zoomLevel)}
        options={ZOOM_OPTIONS}
        onChange={(v) => props.onUpdate({ zoomLevel: parseInt(v) })}
      />
    </div>
  );
};

const EditorSection = (props: { settings: EditorSettings; onUpdate: (partial: Partial<EditorSettings>) => void }) => (
  <div class="settings-section">
    <div class="settings-section-title">Editor</div>
    <NumberField
      label="Font Size"
      value={props.settings.fontSize}
      onChange={(v) => props.onUpdate({ fontSize: v })}
      min={8}
      max={32}
    />
    <TextField
      label="Font Family"
      value={props.settings.fontFamily}
      onChange={(v) => props.onUpdate({ fontFamily: v })}
    />
    <NumberField
      label="Line Height"
      value={props.settings.lineHeight}
      onChange={(v) => props.onUpdate({ lineHeight: v })}
      min={10}
      max={60}
    />
    <NumberField
      label="Tab Size"
      value={props.settings.tabSize}
      onChange={(v) => props.onUpdate({ tabSize: v })}
      min={1}
      max={8}
    />
    <SelectField
      label="Word Wrap"
      value={props.settings.wordWrap}
      options={[
        { value: "off", label: "Off" },
        { value: "on", label: "On" },
        { value: "wordWrapColumn", label: "Word Wrap Column" },
        { value: "bounded", label: "Bounded" },
      ]}
      onChange={(v) => props.onUpdate({ wordWrap: v as EditorSettings["wordWrap"] })}
    />
    <SelectField
      label="Render Whitespace"
      value={props.settings.renderWhitespace}
      options={[
        { value: "none", label: "None" },
        { value: "boundary", label: "Boundary" },
        { value: "selection", label: "Selection" },
        { value: "trailing", label: "Trailing" },
        { value: "all", label: "All" },
      ]}
      onChange={(v) => props.onUpdate({ renderWhitespace: v as EditorSettings["renderWhitespace"] })}
    />
  </div>
);

const ZOOM_OPTIONS = Array.from({ length: 15 }, (_, i) => {
  const level = i - 5;
  const percent = Math.round(Math.pow(1.2, level) * 100);
  return { value: String(level), label: `${percent}%` };
});

const FONT_WEIGHT_OPTIONS = [
  { value: "normal", label: "Normal" },
  { value: "bold", label: "Bold" },
  { value: "100", label: "100" },
  { value: "200", label: "200" },
  { value: "300", label: "300" },
  { value: "400", label: "400" },
  { value: "500", label: "500" },
  { value: "600", label: "600" },
  { value: "700", label: "700" },
  { value: "800", label: "800" },
  { value: "900", label: "900" },
];

const TerminalSection = (props: {
  settings: TerminalSettings;
  onUpdate: (partial: Partial<TerminalSettings>) => void;
  uiSettings: UISettings;
  onUpdateUI: (partial: Partial<UISettings>) => void;
}) => (
  <div class="settings-section">
    <div class="settings-section-title">Terminal</div>

    <div class="settings-subsection-title">General</div>
    <CheckboxField
      label="Always Open Terminal on Start"
      checked={props.uiSettings.alwaysOpenTerminalOnStart}
      onChange={(v) => props.onUpdateUI({ alwaysOpenTerminalOnStart: v })}
    />

    <div class="settings-subsection-title">Text &amp; Font</div>
    <NumberField
      label="Font Size"
      value={props.settings.fontSize}
      onChange={(v) => props.onUpdate({ fontSize: v })}
      min={8}
      max={32}
    />
    <TextField
      label="Font Family"
      value={props.settings.fontFamily}
      onChange={(v) => props.onUpdate({ fontFamily: v })}
    />
    <NumberField
      label="Line Height"
      value={props.settings.lineHeight}
      onChange={(v) => props.onUpdate({ lineHeight: v })}
      min={0.8}
      max={3}
      step={0.1}
    />
    <SelectField
      label="Font Weight"
      value={String(props.settings.fontWeight)}
      options={FONT_WEIGHT_OPTIONS}
      onChange={(v) => props.onUpdate({ fontWeight: v as FontWeight })}
    />
    <SelectField
      label="Font Weight Bold"
      value={String(props.settings.fontWeightBold)}
      options={FONT_WEIGHT_OPTIONS}
      onChange={(v) => props.onUpdate({ fontWeightBold: v as FontWeight })}
    />
    <NumberField
      label="Letter Spacing"
      value={props.settings.letterSpacing}
      onChange={(v) => props.onUpdate({ letterSpacing: v })}
      min={-5}
      max={10}
      step={1}
    />

    <div class="settings-subsection-title">Cursor</div>
    <SelectField
      label="Cursor Style"
      value={props.settings.cursorStyle}
      options={[
        { value: "block", label: "Block" },
        { value: "underline", label: "Underline" },
        { value: "bar", label: "Bar" },
      ]}
      onChange={(v) => props.onUpdate({ cursorStyle: v as TerminalSettings["cursorStyle"] })}
    />
    <CheckboxField
      label="Cursor Blink"
      checked={props.settings.cursorBlink}
      onChange={(v) => props.onUpdate({ cursorBlink: v })}
    />
    <NumberField
      label="Cursor Width"
      value={props.settings.cursorWidth}
      onChange={(v) => props.onUpdate({ cursorWidth: v })}
      min={1}
      max={5}
      step={1}
    />
    <SelectField
      label="Cursor Inactive Style"
      value={props.settings.cursorInactiveStyle}
      options={[
        { value: "outline", label: "Outline" },
        { value: "block", label: "Block" },
        { value: "bar", label: "Bar" },
        { value: "underline", label: "Underline" },
        { value: "none", label: "None" },
      ]}
      onChange={(v) => props.onUpdate({ cursorInactiveStyle: v as TerminalSettings["cursorInactiveStyle"] })}
    />

    <div class="settings-subsection-title">Scrolling</div>
    <NumberField
      label="Scrollback for New Terminals (KiB)"
      value={props.settings.scrollback}
      onChange={(v) => props.onUpdate({ scrollback: v })}
      min={500}
      max={100000}
      step={500}
    />
    <NumberField
      label="Scroll Sensitivity"
      value={props.settings.scrollSensitivity}
      onChange={(v) => props.onUpdate({ scrollSensitivity: v })}
      min={0.1}
      max={10}
      step={0.1}
    />
    <NumberField
      label="Fast Scroll Sensitivity"
      value={props.settings.fastScrollSensitivity}
      onChange={(v) => props.onUpdate({ fastScrollSensitivity: v })}
      min={1}
      max={20}
      step={1}
    />
    <NumberField
      label="Smooth Scroll Duration"
      value={props.settings.smoothScrollDuration}
      onChange={(v) => props.onUpdate({ smoothScrollDuration: v })}
      min={0}
      max={500}
      step={25}
    />
    <CheckboxField
      label="Scroll on User Input"
      checked={props.settings.scrollOnUserInput}
      onChange={(v) => props.onUpdate({ scrollOnUserInput: v })}
    />

    <div class="settings-subsection-title">Behavior</div>
    <CheckboxField
      label="Copy on Select"
      checked={props.settings.copyOnSelect}
      onChange={(v) => props.onUpdate({ copyOnSelect: v })}
    />
    <CheckboxField
      label="Right Click Selects Word"
      checked={props.settings.rightClickSelectsWord}
      onChange={(v) => props.onUpdate({ rightClickSelectsWord: v })}
    />
    <CheckboxField
      label="Alt Click Moves Cursor"
      checked={props.settings.altClickMovesCursor}
      onChange={(v) => props.onUpdate({ altClickMovesCursor: v })}
    />
    <CheckboxField
      label="macOS Option as Meta"
      checked={props.settings.macOptionIsMeta}
      onChange={(v) => props.onUpdate({ macOptionIsMeta: v })}
    />
    <CheckboxField
      label="macOS Option Click Forces Selection"
      checked={props.settings.macOptionClickForcesSelection}
      onChange={(v) => props.onUpdate({ macOptionClickForcesSelection: v })}
    />

    <div class="settings-subsection-title">Rendering</div>
    <CheckboxField
      label="Draw Bold Text in Bright Colors"
      checked={props.settings.drawBoldTextInBrightColors}
      onChange={(v) => props.onUpdate({ drawBoldTextInBrightColors: v })}
    />
    <NumberField
      label="Minimum Contrast Ratio"
      value={props.settings.minimumContrastRatio}
      onChange={(v) => props.onUpdate({ minimumContrastRatio: v })}
      min={1}
      max={21}
      step={0.5}
    />
    <CheckboxField
      label="Rescale Overlapping Glyphs"
      checked={props.settings.rescaleOverlappingGlyphs}
      onChange={(v) => props.onUpdate({ rescaleOverlappingGlyphs: v })}
    />
    <NumberField
      label="Color Saturation"
      value={props.settings.colorSaturation}
      onChange={(v) => props.onUpdate({ colorSaturation: v })}
      min={0.5}
      max={2}
      step={0.01}
    />

    <div class="settings-subsection-title">Shell</div>
    <ShellPathField value={props.settings.shellPath} onChange={(v) => props.onUpdate({ shellPath: v })} />
    <SelectField
      label="Bell Style"
      value={props.settings.bellStyle}
      options={[
        { value: "off", label: "Off" },
        { value: "sound", label: "Sound" },
        { value: "visual", label: "Visual" },
        { value: "both", label: "Both" },
      ]}
      onChange={(v) => props.onUpdate({ bellStyle: v as TerminalSettings["bellStyle"] })}
    />

    <div class="settings-subsection-title">Advanced</div>
    <NumberField
      label="Tab Stop Width"
      value={props.settings.tabStopWidth}
      onChange={(v) => props.onUpdate({ tabStopWidth: v })}
      min={1}
      max={16}
      step={1}
    />
    <TextField
      label="Word Separator"
      value={props.settings.wordSeparator}
      onChange={(v) => props.onUpdate({ wordSeparator: v })}
    />
  </div>
);

const GitSection = (props: { settings: GitSettings; onUpdate: (partial: Partial<GitSettings>) => void }) => {
  const [userName, setUserName] = createSignal("");
  const [userEmail, setUserEmail] = createSignal("");
  let nameTimer: ReturnType<typeof setTimeout> | undefined;
  let emailTimer: ReturnType<typeof setTimeout> | undefined;

  onSettled(() => {
    getGitConfig("user.name")
      .then(setUserName)
      .catch(() => {});
    getGitConfig("user.email")
      .then(setUserEmail)
      .catch(() => {});
    return () => {
      if (nameTimer) clearTimeout(nameTimer);
      if (emailTimer) clearTimeout(emailTimer);
    };
  });

  const handleNameChange = (value: string) => {
    setUserName(value);
    if (nameTimer) clearTimeout(nameTimer);
    nameTimer = setTimeout(() => {
      setGitConfig("user.name", value).catch(() => {});
    }, 500);
  };

  const handleEmailChange = (value: string) => {
    setUserEmail(value);
    if (emailTimer) clearTimeout(emailTimer);
    emailTimer = setTimeout(() => {
      setGitConfig("user.email", value).catch(() => {});
    }, 500);
  };

  return (
    <div class="settings-section">
      <div class="settings-section-title">Git</div>
      <CheckboxField label="Git Blame" checked={props.settings.blame} onChange={(v) => props.onUpdate({ blame: v })} />
      <TextField label="User Name" value={userName()} onChange={handleNameChange} />
      <TextField label="User Email" value={userEmail()} onChange={handleEmailChange} />
    </div>
  );
};

const ProjectsSection = (props: {
  settings: ProjectsSettings;
  onUpdate: (partial: Partial<ProjectsSettings>) => void;
}) => {
  const [showModal, setShowModal] = createSignal(false);
  const id = createUniqueId();

  const displayValue = () =>
    props.settings.workspaces.length > 0
      ? props.settings.workspaces
          .map((ws) => (ws.scanDepth === 1 ? ws.directory : `${ws.directory}:${ws.scanDepth}`))
          .join(", ")
      : "";

  return (
    <div class="settings-section">
      <div class="settings-section-title">Projects</div>
      <div class="settings-field">
        <label class="settings-label" for={id}>
          Workspace Directories
        </label>
        <div class="settings-field-with-action">
          <input
            id={id}
            class="settings-input settings-input-full"
            type="text"
            value={displayValue()}
            placeholder="Not configured"
            readonly
          />
          <button class="settings-reset-btn" onClick={() => setShowModal(true)}>
            Configure...
          </button>
        </div>
      </div>
      <Show when={showModal()}>
        <WorkspaceConfigModal
          onClose={() => setShowModal(false)}
          workspaces={props.settings.workspaces}
          onSave={(workspaces) => props.onUpdate({ workspaces })}
        />
      </Show>
    </div>
  );
};

const CUSTOM_SHELL = "__custom__";

const SHELL_PRESETS: { value: string; label: string; platforms: string[] }[] = [
  { value: "", label: "Default ($SHELL)", platforms: ["mac", "linux", "win"] },
  { value: "/bin/zsh", label: "Zsh (/bin/zsh)", platforms: ["mac", "linux"] },
  { value: "/bin/bash", label: "Bash (/bin/bash)", platforms: ["mac", "linux"] },
  { value: "/usr/bin/fish", label: "Fish (/usr/bin/fish)", platforms: ["linux"] },
  { value: "/opt/homebrew/bin/fish", label: "Fish (/opt/homebrew/bin/fish)", platforms: ["mac"] },
  { value: "/bin/sh", label: "sh (/bin/sh)", platforms: ["mac", "linux"] },
  { value: "powershell.exe", label: "PowerShell", platforms: ["win"] },
  { value: "cmd.exe", label: "CMD", platforms: ["win"] },
  { value: "wsl.exe", label: "WSL (Ubuntu)", platforms: ["win"] },
  { value: "C:\\Program Files\\Git\\bin\\bash.exe", label: "Git Bash", platforms: ["win"] },
];

const getPlatform = (): string => {
  if (PLATFORM === "macos") return "mac";
  if (PLATFORM === "windows") return "win";
  return "linux";
};

const ShellPathField = (props: { value: string; onChange: (v: string) => void }) => {
  const platform = getPlatform();
  const options = SHELL_PRESETS.filter((s) => s.platforms.includes(platform));
  const [customMode, setCustomMode] = createSignal(untrack(() => !options.some((o) => o.value === props.value)));

  const selectValue = () => (customMode() ? CUSTOM_SHELL : props.value);

  return (
    <label class="settings-field">
      <span class="settings-label">Shell Path</span>
      <div class="settings-field-shell">
        <select
          class="settings-input settings-select"
          value={selectValue()}
          onChange={(e) => {
            const next = (e.currentTarget as HTMLSelectElement).value;
            if (next === CUSTOM_SHELL) {
              setCustomMode(true);
              props.onChange("");
            } else {
              setCustomMode(false);
              props.onChange(next);
            }
          }}
        >
          <For each={options} keyed={(opt) => opt.value}>
            {(opt) => <option value={opt().value}>{opt().label}</option>}
          </For>
          <option value={CUSTOM_SHELL}>Custom...</option>
        </select>
        <Show when={customMode()}>
          <input
            class="settings-input"
            type="text"
            value={props.value}
            placeholder="/path/to/shell"
            onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => props.onChange(e.currentTarget.value)}
          />
        </Show>
      </div>
    </label>
  );
};

const NumberField = (props: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min?: number;
  max?: number;
  step?: number;
}) => (
  <label class="settings-field">
    <span class="settings-label">{props.label}</span>
    <input
      class="settings-input settings-input-number"
      type="number"
      value={props.value}
      min={props.min}
      max={props.max}
      step={props.step ?? 1}
      onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => {
        const v = parseFloat(e.currentTarget.value);
        if (!isNaN(v)) props.onChange(v);
      }}
    />
  </label>
);

const TextField = (props: { label: string; value: string; onChange: (v: string) => void }) => (
  <label class="settings-field">
    <span class="settings-label">{props.label}</span>
    <input
      class="settings-input"
      type="text"
      value={props.value}
      onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => props.onChange(e.currentTarget.value)}
    />
  </label>
);

const SelectField = (props: {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
}) => (
  <label class="settings-field">
    <span class="settings-label">{props.label}</span>
    <select
      class="settings-input settings-select"
      value={props.value}
      onChange={(e) => props.onChange((e.currentTarget as HTMLSelectElement).value)}
    >
      <For each={props.options} keyed={(opt) => opt.value}>
        {(opt) => <option value={opt().value}>{opt().label}</option>}
      </For>
    </select>
  </label>
);

const CheckboxField = (props: { label: string; checked: boolean; onChange: (v: boolean) => void }) => {
  const id = createUniqueId();
  return (
    <div class="settings-field">
      <label class="settings-label" for={id}>
        {props.label}
      </label>
      <button
        id={id}
        class={["settings-toggle", { active: props.checked }]}
        onClick={() => props.onChange(!props.checked)}
        role="switch"
        aria-checked={props.checked ? "true" : "false"}
      >
        <span class="settings-toggle-knob" />
      </button>
    </div>
  );
};
