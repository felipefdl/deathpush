import { useEffect, useMemo, useRef, useState } from "react";
import { useSettingsStore } from "../../stores/settings-store";
import type { EditorSettings, GitSettings, ProjectsSettings, TerminalSettings, UISettings } from "../../stores/settings-store";
import type { FontWeight } from "@xterm/xterm";
import { useThemeStore } from "../../stores/theme-store";
import { useIconThemeStore } from "../../stores/icon-theme-store";
import { THEME_ENTRIES } from "../../lib/themes/theme-registry";
import { getGitConfig, setGitConfig } from "../../lib/tauri-commands";
import { WorkspaceConfigModal } from "../shared/workspace-config-modal";
import "../../styles/settings.css";

export const SettingsPage = () => {
  const { settings, updateUI, updateEditor, updateTerminal, updateGit, updateProjects, resetToDefaults } = useSettingsStore();

  return (
    <div className="settings-page">
      <div className="settings-header">
        <span className="settings-title">Settings</span>
        <button className="settings-reset-btn" onClick={resetToDefaults}>
          Reset to Defaults
        </button>
      </div>
      <div className="settings-content">
        <AppearanceSection settings={settings.ui} onUpdate={updateUI} />
        <EditorSection settings={settings.editor} onUpdate={updateEditor} />
        <GitSection settings={settings.git} onUpdate={updateGit} />
        <ProjectsSection settings={settings.projects} onUpdate={updateProjects} />
        <TerminalSection settings={settings.terminal} onUpdate={updateTerminal} uiSettings={settings.ui} onUpdateUI={updateUI} />
      </div>
    </div>
  );
};

const AppearanceSection = ({
  settings,
  onUpdate,
}: {
  settings: UISettings;
  onUpdate: (partial: Partial<UISettings>) => void;
}) => {
  const { currentTheme, preferredDarkThemeId, preferredLightThemeId, setPreferredDarkTheme, setPreferredLightTheme } =
    useThemeStore();
  const { currentIconTheme } = useIconThemeStore();

  const darkThemes = THEME_ENTRIES.filter((t) => t.kind === "dark" || t.kind === "hc-dark");
  const lightThemes = THEME_ENTRIES.filter((t) => t.kind === "light" || t.kind === "hc-light");

  return (
    <div className="settings-section">
      <div className="settings-section-title">Appearance</div>
      <div className="settings-field">
        <label className="settings-label">Color Theme</label>
        <button
          className="settings-input settings-picker-btn"
          onClick={() => window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"))}
        >
          {currentTheme.label}
          <span className="settings-picker-hint">Cmd+K Cmd+T</span>
        </button>
      </div>
      <SelectField
        label="Preferred Dark Theme"
        value={preferredDarkThemeId}
        options={darkThemes.map((t) => ({ value: t.id, label: t.label }))}
        onChange={setPreferredDarkTheme}
      />
      <SelectField
        label="Preferred Light Theme"
        value={preferredLightThemeId}
        options={lightThemes.map((t) => ({ value: t.id, label: t.label }))}
        onChange={setPreferredLightTheme}
      />
      <div className="settings-field">
        <label className="settings-label">File Icon Theme</label>
        <button
          className="settings-input settings-picker-btn"
          onClick={() => window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"))}
        >
          {currentIconTheme.label}
          <span className="settings-picker-hint">Cmd+K Cmd+I</span>
        </button>
      </div>
      <SelectField
        label="Sidebar Position"
        value={settings.sidebarPosition}
        options={[
          { value: "left", label: "Left" },
          { value: "right", label: "Right" },
        ]}
        onChange={(v) => onUpdate({ sidebarPosition: v as UISettings["sidebarPosition"] })}
      />
      <TextField label="UI Font Family" value={settings.fontFamily} onChange={(v) => onUpdate({ fontFamily: v })} />
      <NumberField label="UI Font Size" value={settings.fontSize} onChange={(v) => onUpdate({ fontSize: v })} min={10} max={20} />
      <SelectField
        label="Zoom"
        value={String(settings.zoomLevel)}
        options={ZOOM_OPTIONS}
        onChange={(v) => onUpdate({ zoomLevel: parseInt(v) })}
      />
    </div>
  );
};

const EditorSection = ({
  settings,
  onUpdate,
}: {
  settings: EditorSettings;
  onUpdate: (partial: Partial<EditorSettings>) => void;
}) => (
  <div className="settings-section">
    <div className="settings-section-title">Editor</div>
    <NumberField label="Font Size" value={settings.fontSize} onChange={(v) => onUpdate({ fontSize: v })} min={8} max={32} />
    <TextField label="Font Family" value={settings.fontFamily} onChange={(v) => onUpdate({ fontFamily: v })} />
    <NumberField label="Line Height" value={settings.lineHeight} onChange={(v) => onUpdate({ lineHeight: v })} min={10} max={60} />
    <NumberField label="Tab Size" value={settings.tabSize} onChange={(v) => onUpdate({ tabSize: v })} min={1} max={8} />
    <SelectField
      label="Word Wrap"
      value={settings.wordWrap}
      options={[
        { value: "off", label: "Off" },
        { value: "on", label: "On" },
        { value: "wordWrapColumn", label: "Word Wrap Column" },
        { value: "bounded", label: "Bounded" },
      ]}
      onChange={(v) => onUpdate({ wordWrap: v as EditorSettings["wordWrap"] })}
    />
    <SelectField
      label="Render Whitespace"
      value={settings.renderWhitespace}
      options={[
        { value: "none", label: "None" },
        { value: "boundary", label: "Boundary" },
        { value: "selection", label: "Selection" },
        { value: "trailing", label: "Trailing" },
        { value: "all", label: "All" },
      ]}
      onChange={(v) => onUpdate({ renderWhitespace: v as EditorSettings["renderWhitespace"] })}
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

const TerminalSection = ({
  settings,
  onUpdate,
  uiSettings,
  onUpdateUI,
}: {
  settings: TerminalSettings;
  onUpdate: (partial: Partial<TerminalSettings>) => void;
  uiSettings: UISettings;
  onUpdateUI: (partial: Partial<UISettings>) => void;
}) => (
  <div className="settings-section">
    <div className="settings-section-title">Terminal</div>

    <div className="settings-subsection-title">General</div>
    <CheckboxField
      label="Always Open Terminal on Start"
      checked={uiSettings.alwaysOpenTerminalOnStart}
      onChange={(v) => onUpdateUI({ alwaysOpenTerminalOnStart: v })}
    />

    <div className="settings-subsection-title">Text &amp; Font</div>
    <NumberField label="Font Size" value={settings.fontSize} onChange={(v) => onUpdate({ fontSize: v })} min={8} max={32} />
    <TextField label="Font Family" value={settings.fontFamily} onChange={(v) => onUpdate({ fontFamily: v })} />
    <NumberField
      label="Line Height"
      value={settings.lineHeight}
      onChange={(v) => onUpdate({ lineHeight: v })}
      min={0.8}
      max={3}
      step={0.1}
    />
    <SelectField
      label="Font Weight"
      value={String(settings.fontWeight)}
      options={FONT_WEIGHT_OPTIONS}
      onChange={(v) => onUpdate({ fontWeight: v as FontWeight })}
    />
    <SelectField
      label="Font Weight Bold"
      value={String(settings.fontWeightBold)}
      options={FONT_WEIGHT_OPTIONS}
      onChange={(v) => onUpdate({ fontWeightBold: v as FontWeight })}
    />
    <NumberField
      label="Letter Spacing"
      value={settings.letterSpacing}
      onChange={(v) => onUpdate({ letterSpacing: v })}
      min={-5}
      max={10}
      step={1}
    />

    <div className="settings-subsection-title">Cursor</div>
    <SelectField
      label="Cursor Style"
      value={settings.cursorStyle}
      options={[
        { value: "block", label: "Block" },
        { value: "underline", label: "Underline" },
        { value: "bar", label: "Bar" },
      ]}
      onChange={(v) => onUpdate({ cursorStyle: v as TerminalSettings["cursorStyle"] })}
    />
    <CheckboxField label="Cursor Blink" checked={settings.cursorBlink} onChange={(v) => onUpdate({ cursorBlink: v })} />
    <NumberField
      label="Cursor Width"
      value={settings.cursorWidth}
      onChange={(v) => onUpdate({ cursorWidth: v })}
      min={1}
      max={5}
      step={1}
    />
    <SelectField
      label="Cursor Inactive Style"
      value={settings.cursorInactiveStyle}
      options={[
        { value: "outline", label: "Outline" },
        { value: "block", label: "Block" },
        { value: "bar", label: "Bar" },
        { value: "underline", label: "Underline" },
        { value: "none", label: "None" },
      ]}
      onChange={(v) => onUpdate({ cursorInactiveStyle: v as TerminalSettings["cursorInactiveStyle"] })}
    />

    <div className="settings-subsection-title">Scrolling</div>
    <NumberField
      label="Scrollback"
      value={settings.scrollback}
      onChange={(v) => onUpdate({ scrollback: v })}
      min={500}
      max={100000}
      step={500}
    />
    <NumberField
      label="Scroll Sensitivity"
      value={settings.scrollSensitivity}
      onChange={(v) => onUpdate({ scrollSensitivity: v })}
      min={0.1}
      max={10}
      step={0.1}
    />
    <NumberField
      label="Fast Scroll Sensitivity"
      value={settings.fastScrollSensitivity}
      onChange={(v) => onUpdate({ fastScrollSensitivity: v })}
      min={1}
      max={20}
      step={1}
    />
    <NumberField
      label="Smooth Scroll Duration"
      value={settings.smoothScrollDuration}
      onChange={(v) => onUpdate({ smoothScrollDuration: v })}
      min={0}
      max={500}
      step={25}
    />
    <CheckboxField
      label="Scroll on User Input"
      checked={settings.scrollOnUserInput}
      onChange={(v) => onUpdate({ scrollOnUserInput: v })}
    />

    <div className="settings-subsection-title">Behavior</div>
    <CheckboxField label="Copy on Select" checked={settings.copyOnSelect} onChange={(v) => onUpdate({ copyOnSelect: v })} />
    <CheckboxField
      label="Right Click Selects Word"
      checked={settings.rightClickSelectsWord}
      onChange={(v) => onUpdate({ rightClickSelectsWord: v })}
    />
    <CheckboxField
      label="Alt Click Moves Cursor"
      checked={settings.altClickMovesCursor}
      onChange={(v) => onUpdate({ altClickMovesCursor: v })}
    />
    <CheckboxField
      label="macOS Option as Meta"
      checked={settings.macOptionIsMeta}
      onChange={(v) => onUpdate({ macOptionIsMeta: v })}
    />
    <CheckboxField
      label="macOS Option Click Forces Selection"
      checked={settings.macOptionClickForcesSelection}
      onChange={(v) => onUpdate({ macOptionClickForcesSelection: v })}
    />

    <div className="settings-subsection-title">Rendering</div>
    <CheckboxField
      label="Draw Bold Text in Bright Colors"
      checked={settings.drawBoldTextInBrightColors}
      onChange={(v) => onUpdate({ drawBoldTextInBrightColors: v })}
    />
    <NumberField
      label="Minimum Contrast Ratio"
      value={settings.minimumContrastRatio}
      onChange={(v) => onUpdate({ minimumContrastRatio: v })}
      min={1}
      max={21}
      step={0.5}
    />
    <CheckboxField
      label="Rescale Overlapping Glyphs"
      checked={settings.rescaleOverlappingGlyphs}
      onChange={(v) => onUpdate({ rescaleOverlappingGlyphs: v })}
    />

    <div className="settings-subsection-title">Shell</div>
    <ShellPathField value={settings.shellPath} onChange={(v) => onUpdate({ shellPath: v })} />
    <SelectField
      label="Bell Style"
      value={settings.bellStyle}
      options={[
        { value: "off", label: "Off" },
        { value: "sound", label: "Sound" },
        { value: "visual", label: "Visual" },
        { value: "both", label: "Both" },
      ]}
      onChange={(v) => onUpdate({ bellStyle: v as TerminalSettings["bellStyle"] })}
    />

    <div className="settings-subsection-title">Advanced</div>
    <NumberField
      label="Tab Stop Width"
      value={settings.tabStopWidth}
      onChange={(v) => onUpdate({ tabStopWidth: v })}
      min={1}
      max={16}
      step={1}
    />
    <TextField label="Word Separator" value={settings.wordSeparator} onChange={(v) => onUpdate({ wordSeparator: v })} />
  </div>
);

const GitSection = ({
  settings,
  onUpdate,
}: {
  settings: GitSettings;
  onUpdate: (partial: Partial<GitSettings>) => void;
}) => {
  const [userName, setUserName] = useState("");
  const [userEmail, setUserEmail] = useState("");
  const nameTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const emailTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    getGitConfig("user.name").then(setUserName).catch(() => {});
    getGitConfig("user.email").then(setUserEmail).catch(() => {});
  }, []);

  const handleNameChange = (value: string) => {
    setUserName(value);
    if (nameTimerRef.current) clearTimeout(nameTimerRef.current);
    nameTimerRef.current = setTimeout(() => {
      setGitConfig("user.name", value).catch(() => {});
    }, 500);
  };

  const handleEmailChange = (value: string) => {
    setUserEmail(value);
    if (emailTimerRef.current) clearTimeout(emailTimerRef.current);
    emailTimerRef.current = setTimeout(() => {
      setGitConfig("user.email", value).catch(() => {});
    }, 500);
  };

  return (
    <div className="settings-section">
      <div className="settings-section-title">Git</div>
      <CheckboxField label="Git Blame" checked={settings.blame} onChange={(v) => onUpdate({ blame: v })} />
      <TextField label="User Name" value={userName} onChange={handleNameChange} />
      <TextField label="User Email" value={userEmail} onChange={handleEmailChange} />
    </div>
  );
};

const ProjectsSection = ({
  settings,
  onUpdate,
}: {
  settings: ProjectsSettings;
  onUpdate: (partial: Partial<ProjectsSettings>) => void;
}) => {
  const [showModal, setShowModal] = useState(false);

  const displayValue = settings.workspaces.length > 0
    ? settings.workspaces.map((ws) => ws.scanDepth === 1 ? ws.directory : `${ws.directory}:${ws.scanDepth}`).join(", ")
    : "";

  return (
    <div className="settings-section">
      <div className="settings-section-title">Projects</div>
      <div className="settings-field">
        <label className="settings-label">Workspace Directories</label>
        <div className="settings-field-with-action">
          <input
            className="settings-input settings-input-full"
            type="text"
            value={displayValue}
            placeholder="Not configured"
            readOnly
          />
          <button className="settings-reset-btn" onClick={() => setShowModal(true)}>
            Configure...
          </button>
        </div>
      </div>
      {showModal && (
        <WorkspaceConfigModal
          onClose={() => setShowModal(false)}
          workspaces={settings.workspaces}
          onSave={(workspaces) => onUpdate({ workspaces })}
        />
      )}
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

import { PLATFORM } from "../../lib/platform";

const getPlatform = (): string => {
  if (PLATFORM === "macos") return "mac";
  if (PLATFORM === "windows") return "win";
  return "linux";
};

const ShellPathField = ({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) => {
  const platform = useMemo(getPlatform, []);
  const options = useMemo(
    () => SHELL_PRESETS.filter((s) => s.platforms.includes(platform)),
    [platform],
  );
  const isPreset = options.some((o) => o.value === value);
  const [customMode, setCustomMode] = useState(!isPreset);

  const selectValue = customMode ? CUSTOM_SHELL : value;

  return (
    <div className="settings-field">
      <label className="settings-label">Shell Path</label>
      <div className="settings-field-shell">
        <select
          className="settings-input settings-select"
          value={selectValue}
          onChange={(e) => {
            if (e.target.value === CUSTOM_SHELL) {
              setCustomMode(true);
              onChange("");
            } else {
              setCustomMode(false);
              onChange(e.target.value);
            }
          }}
        >
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
          <option value={CUSTOM_SHELL}>Custom...</option>
        </select>
        {customMode && (
          <input
            className="settings-input"
            type="text"
            value={value}
            placeholder="/path/to/shell"
            onChange={(e) => onChange(e.target.value)}
          />
        )}
      </div>
    </div>
  );
};

const NumberField = ({
  label,
  value,
  onChange,
  min,
  max,
  step = 1,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min?: number;
  max?: number;
  step?: number;
}) => (
  <div className="settings-field">
    <label className="settings-label">{label}</label>
    <input
      className="settings-input settings-input-number"
      type="number"
      value={value}
      min={min}
      max={max}
      step={step}
      onChange={(e) => {
        const v = parseFloat(e.target.value);
        if (!isNaN(v)) onChange(v);
      }}
    />
  </div>
);

const TextField = ({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) => (
  <div className="settings-field">
    <label className="settings-label">{label}</label>
    <input
      className="settings-input"
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  </div>
);

const SelectField = ({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
}) => (
  <div className="settings-field">
    <label className="settings-label">{label}</label>
    <select
      className="settings-input settings-select"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>{opt.label}</option>
      ))}
    </select>
  </div>
);

const CheckboxField = ({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) => (
  <div className="settings-field">
    <label className="settings-label">{label}</label>
    <button
      className={`settings-toggle${checked ? " active" : ""}`}
      onClick={() => onChange(!checked)}
      role="switch"
      aria-checked={checked}
    >
      <span className="settings-toggle-knob" />
    </button>
  </div>
);
