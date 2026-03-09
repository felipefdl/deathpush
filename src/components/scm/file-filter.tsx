import { useState, useCallback, useEffect, useRef } from "react";
import { useRepositoryStore } from "../../stores/repository-store";

export const FileFilter = () => {
  const { setFileFilter } = useRepositoryStore();
  const [value, setValue] = useState("");
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setValue(val);
    clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setFileFilter(val);
    }, 150);
  }, [setFileFilter]);

  useEffect(() => {
    return () => clearTimeout(timerRef.current);
  }, []);

  return (
    <div className="file-filter">
      <span className="codicon codicon-search file-filter-icon" />
      <input
        className="file-filter-input"
        type="search"
        placeholder="Filter files..."
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
        data-form-type="other"
        value={value}
        onChange={handleChange}
      />
      {value && (
        <button className="file-filter-clear" onClick={() => { setValue(""); setFileFilter(""); }}>
          <span className="codicon codicon-close" />
        </button>
      )}
    </div>
  );
};
