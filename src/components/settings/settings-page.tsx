import { useEffect, useRef, useState } from "react";
import { useSettingsStore } from "../../stores/settings-store";
import type { EditorSettings, GitSettings, ProjectsSettings, TerminalSettings, UISettings } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";
import { useIconThemeStore } from "../../stores/icon-theme-store";
import { THEME_ENTRIES } from "../../lib/themes/theme-registry";
import { open } from "@tauri-apps/plugin-dialog";
import { getGitConfig, setGitConfig } from "../../lib/tauri-commands";
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
        <TerminalSection settings={settings.terminal} onUpdate={updateTerminal} />
        <GitSection settings={settings.git} onUpdate={updateGit} />
        <ProjectsSection settings={settings.projects} onUpdate={updateProjects} />
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
    <CheckboxField label="Minimap" checked={settings.minimap} onChange={(v) => onUpdate({ minimap: v })} />
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

const TerminalSection = ({
  settings,
  onUpdate,
}: {
  settings: TerminalSettings;
  onUpdate: (partial: Partial<TerminalSettings>) => void;
}) => (
  <div className="settings-section">
    <div className="settings-section-title">Terminal</div>
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
    <CheckboxField label="Cursor Blink" checked={settings.cursorBlink} onChange={(v) => onUpdate({ cursorBlink: v })} />
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
  const handleBrowse = async () => {
    const selected = await open({ directory: true, title: "Select Git Projects Directory" });
    if (selected) {
      onUpdate({ projectsDirectory: selected });
    }
  };

  return (
    <div className="settings-section">
      <div className="settings-section-title">Projects</div>
      <div className="settings-field">
        <label className="settings-label">Git Projects Directory</label>
        <div style={{ display: "flex", gap: 6, flex: 1, maxWidth: 300 }}>
          <input
            className="settings-input"
            type="text"
            value={settings.projectsDirectory}
            placeholder="Not configured"
            onChange={(e) => onUpdate({ projectsDirectory: e.target.value })}
            style={{ flex: 1, maxWidth: "none" }}
          />
          <button className="settings-reset-btn" onClick={handleBrowse}>
            Browse...
          </button>
        </div>
      </div>
      <NumberField
        label="Scan Depth"
        value={settings.scanDepth}
        onChange={(v) => onUpdate({ scanDepth: v })}
        min={1}
        max={5}
      />
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
