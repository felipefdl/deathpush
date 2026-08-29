import { createSignal, onSettled } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";

export const FileFilter = () => {
  const { setFileFilter } = repositoryStore.getState();
  const [value, setValue] = createSignal("");
  let timer: ReturnType<typeof setTimeout> | undefined;

  const handleInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    const val = e.currentTarget.value;
    setValue(val);
    clearTimeout(timer);
    timer = setTimeout(() => {
      setFileFilter(val);
    }, 150);
  };

  onSettled(() => {
    return () => clearTimeout(timer);
  });

  return (
    <div class="file-filter">
      <span class="codicon codicon-search file-filter-icon" />
      <input
        class="file-filter-input"
        type="search"
        placeholder="Filter files..."
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck={false}
        data-form-type="other"
        value={value()}
        onInput={handleInput}
      />
      {value() && (
        <button
          class="file-filter-clear"
          onClick={() => {
            setValue("");
            setFileFilter("");
          }}
        >
          <span class="codicon codicon-close" />
        </button>
      )}
    </div>
  );
};
