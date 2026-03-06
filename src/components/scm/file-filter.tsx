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
      <span className="codicon codicon-search" />
      <input
        className="file-filter-input"
        type="text"
        placeholder="Filter files..."
        value={value}
        onChange={handleChange}
      />
    </div>
  );
};
