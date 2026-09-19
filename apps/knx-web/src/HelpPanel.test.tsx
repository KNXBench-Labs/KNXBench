/** Tests the help panel: a named dialog, a topic list, and every topic's paragraphs on screen. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import HelpPanel from "./HelpPanel";
import { HELP_TOPICS, helpTopicParagraphs } from "./help";
import { messages as enMessages } from "./messages/en";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
});

async function render(onClose = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<HelpPanel onClose={onClose} />);
  });
  return { root, onClose };
}

const dialog = () => host!.querySelector('[role="dialog"]') as HTMLElement;
const topicButtons = () =>
  Array.from(host!.querySelectorAll<HTMLButtonElement>(".help-panel-topics button"));
const paragraphs = () =>
  Array.from(host!.querySelectorAll(".help-panel-topic p")).map((p) => p.textContent);

describe("HelpPanel", () => {
  it("is a dialog labelled by a heading that is actually on screen", async () => {
    await render();
    const labelledBy = dialog().getAttribute("aria-labelledby");
    expect(labelledBy).toBe("help-panel-title");
    const heading = host!.querySelector("#help-panel-title") as HTMLElement;
    expect(heading.tagName).toBe("H2");
    expect(heading.textContent).toBe(enMessages["help.title"]);
  });

  it("lists every topic and marks the open one", async () => {
    await render();
    expect(topicButtons().map((b) => b.textContent)).toEqual(
      HELP_TOPICS.map((t) => enMessages[t.titleKey]),
    );
    const current = topicButtons().filter((b) => b.getAttribute("aria-current") === "page");
    expect(current).toHaveLength(1);
    expect(current[0].textContent).toBe(enMessages[HELP_TOPICS[0].titleKey]);
  });

  it("opens on the first topic and shows its paragraphs", async () => {
    await render();
    const expected = helpTopicParagraphs(HELP_TOPICS[0].id)!.map((k) => enMessages[k]);
    expect(paragraphs()).toEqual(expected);
  });

  // Every topic, not just one: a topic added with a paragraph key the panel
  // never reaches would otherwise pass unnoticed, and the topic list is the
  // only place these keys are named.
  it("renders every topic's paragraphs when that topic is chosen", async () => {
    await render();
    for (const [index, topic] of HELP_TOPICS.entries()) {
      await act(async () => {
        topicButtons()[index].click();
      });
      const expected = helpTopicParagraphs(topic.id)!.map((k) => enMessages[k]);
      expect(paragraphs()).toEqual(expected);
      expect(host!.querySelector("#help-panel-topic-title")!.textContent).toBe(
        enMessages[topic.titleKey],
      );
    }
  });

  it("closes the four concept topics with the shared standard note and the others without it", async () => {
    await render();
    for (const [index, topic] of HELP_TOPICS.entries()) {
      await act(async () => {
        topicButtons()[index].click();
      });
      const last = paragraphs().at(-1);
      if (topic.standardNote) {
        expect(last).toBe(enMessages["help.standardNote"]);
      } else {
        expect(last).not.toBe(enMessages["help.standardNote"]);
      }
    }
  });

  it("keeps the prose region reachable by keyboard", async () => {
    await render();
    const article = host!.querySelector(".help-panel-topic") as HTMLElement;
    expect(article.getAttribute("tabindex")).toBe("0");
    expect(article.getAttribute("aria-labelledby")).toBe("help-panel-topic-title");
  });

  it("closes on the footer button and on Escape", async () => {
    const { onClose } = await render();
    const close = host!.querySelector(".help-panel-footer button") as HTMLButtonElement;
    expect(close.textContent).toBe(enMessages["help.close"]);
    await act(async () => {
      close.click();
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    await act(async () => {
      dialog().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
