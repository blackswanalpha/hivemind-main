interface Props {
  onPick: (text: string) => void;
}

const PROMPTS = [
  { icon: "✦", text: "Summarize this tab" },
  { icon: "⚲", text: "What have I read about Rust async?" },
  { icon: "⤴", text: "Open the docs I bookmarked yesterday" },
  { icon: "⚙", text: "Compare two tabs side by side" },
];

export function StarterPrompts({ onPick }: Props) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-6 text-center">
      <div className="mb-4 text-2xl text-hivemind-accent">✦</div>
      <h3 className="mb-2 text-sm font-medium text-hivemind-fg">
        Hi — ask me about what you&apos;ve read,
        <br />
        or what to do next.
      </h3>
      <p className="mb-6 text-xs text-hivemind-mute">
        Tap a prompt to fill the composer; nothing fires until you press send.
      </p>
      <div className="flex w-full max-w-[320px] flex-col gap-2">
        {PROMPTS.map((p) => (
          <button
            key={p.text}
            onClick={() => onPick(p.text)}
            className="flex w-full items-center gap-2 rounded border border-hivemind-border bg-hivemind-bg/40 px-3 py-2 text-left text-xs text-hivemind-fg hover:border-hivemind-accent hover:bg-hivemind-accent/10"
          >
            <span className="text-hivemind-accent">{p.icon}</span>
            <span>{p.text}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
