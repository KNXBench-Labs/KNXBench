/** The help panel: KNX concepts and this application's surfaces, one topic at a time. */
// ADR-0024's long-form mechanism, opened by F1, the toolbar and the
// command palette. Built on `Overlay`, so the dialog role, the focus
// trap, Escape and focus restoration are the shell's problem and not
// this file's.

import { useEffect, useRef, useState } from "react";
import { DEFAULT_HELP_TOPIC_ID, HELP_TOPICS, helpTopicParagraphs } from "./help";
import type { HelpTopicId } from "./help";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";

export default function HelpPanel(props: { onClose: () => void; initialTopicId?: HelpTopicId }) {
  const { onClose, initialTopicId = DEFAULT_HELP_TOPIC_ID } = props;
  const t = useTranslate();
  const [topicId, setTopicId] = useState<HelpTopicId>(initialTopicId);
  const proseRef = useRef<HTMLElement>(null);
  useEffect(() => {
    const prose = proseRef.current;
    if (prose) { prose.scrollTop = 0; prose.focus(); }
  }, [topicId]);
  // `HELP_TOPICS` is never empty and `topicId` only ever comes from a
  // button built out of it, so the fallback is unreachable — it exists so
  // this component has no non-null assertion in it.
  const topic = HELP_TOPICS.find((entry) => entry.id === topicId) ?? HELP_TOPICS[0];
  const paragraphs = helpTopicParagraphs(topic.id) ?? [];

  return (
    <Overlay labelledBy="help-panel-topic-title" className="help-panel" onClose={onClose}>
      <header className="help-panel-header">
        <h2 id="help-panel-title">{t("help.title")}</h2>
        <p className="help-panel-intro">{t("help.intro")}</p>
      </header>
      <div className="help-panel-body">
        <nav className="help-panel-topics" aria-label={t("help.topics")}>
          {HELP_TOPICS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              aria-current={entry.id === topic.id ? "page" : undefined}
              onClick={() => setTopicId(entry.id)}
            >
              {t(entry.titleKey)}
            </button>
          ))}
        </nav>
        {/* `tabIndex={0}` because this region scrolls, and a scrollable
            region no keyboard can reach is a region a keyboard user cannot
            read. It lands in `Overlay`'s focus trap by the same attribute. */}
        <article
          className="help-panel-topic"
          ref={proseRef}
          aria-labelledby="help-panel-topic-title"
          tabIndex={0}
        >
          <h3 id="help-panel-topic-title">{t(topic.titleKey)}</h3>
          {paragraphs.map((key) => (
            <p key={key}>{t(key)}</p>
          ))}
        </article>
      </div>
      <footer className="help-panel-footer">
        <button type="button" onClick={onClose}>
          {t("help.close")}
        </button>
      </footer>
    </Overlay>
  );
}
