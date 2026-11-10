/** Two-option switch with a sliding thumb. */
export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: [T, string][]; onChange: (v: T) => void }) {
  const index = Math.max(0, options.findIndex(([v]) => v === value));
  return (
    <div className="segmented" role="tablist" style={{ '--seg-index': index, '--seg-count': options.length } as React.CSSProperties}>
      <span className="segmented-thumb" aria-hidden />
      {options.map(([v, label]) => (
        <button key={v} type="button" role="tab" aria-selected={v === value} className={v === value ? 'active' : ''} onClick={() => onChange(v)}>{label}</button>
      ))}
    </div>
  );
}
