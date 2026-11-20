// @vitest-environment node
import { afterEach, describe, expect, it, vi } from "vitest";
import { AIError, editSlide, generateDeck } from "../src/ai";
import { newDeck, starterSlide } from "../src/slides";

const blankAiSlide = {
  layout: "bullets",
  title: "",
  subtitle: "",
  body: "",
  bullets: [],
  columns: [],
  stats: [],
  quote: "",
  attribution: "",
  chart: { kind: "bar", labels: [], values: [], unit: "" },
  table: { headers: [], rows: [] },
  timeline: [],
  imagePrompt: "",
  notes: "",
};

function mockApi(payload: unknown, stop = "end_turn", status = 200) {
  const calls: { url: string; body: any; headers: Headers }[] = [];
  const fetchMock = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), body: JSON.parse(String(init?.body)), headers: new Headers(init?.headers) });
    const message = {
      id: "msg_1",
      type: "message",
      role: "assistant",
      model: "claude-opus-5-5",
      content: [{ type: "text", text: JSON.stringify(payload) }],
      stop_reason: stop,
      stop_sequence: null,
      usage: { input_tokens: 10, output_tokens: 10 },
    };
    const body = status === 200 ? message : { type: "error", error: { type: "authentication_error", message: "bad key" } };
    return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", "request-id": "req_1" } });
  });
  vi.stubGlobal("fetch", fetchMock);
  return calls;
}

afterEach(() => vi.unstubAllGlobals());

const settings = { apiKey: "sk-ant-test", model: "claude-opus-5-5" };

describe("generateDeck", () => {
  it("sends a structured-output request and returns normalized slides", async () => {
    const calls = mockApi({
      title: "Q3 Review",
      slides: [
        { ...blankAiSlide, layout: "title", title: "Q3 Review", subtitle: "Growth team" },
        { ...blankAiSlide, layout: "stats", title: "Results", stats: [{ value: "12%", label: "over target" }] },
        { ...blankAiSlide, layout: "image", title: "Team", imagePrompt: "team photo" },
      ],
    });
    const out = await generateDeck({ prompt: "quarterly review", source: "", slideCount: 3, tone: "Formal", audience: "execs", attachments: [{ name: "a.pdf", kind: "pdf", data: "JVBERi0=" }, { name: "n.md", kind: "text", data: "Revenue up" }] }, settings);

    expect(out.title).toBe("Q3 Review");
    expect(out.slides.map((s) => s.layout)).toEqual(["title", "stats", "image"]);
    expect(out.slides[2].image.prompt).toBe("team photo");
    expect(out.slides[0].id).toMatch(/^s_/);

    const { body, headers, url } = calls[0];
    expect(url).toContain("/v1/messages");
    expect(body.model).toBe("claude-opus-5-5");
    expect(body.output_config.format.type).toBe("json_schema");
    expect(body.output_config.effort).toBe("medium");
    expect(body.fallbacks).toBe("default");
    expect(headers.get("anthropic-beta")).toContain("server-side-fallback-2026-07-01");
    expect(headers.get("anthropic-dangerous-direct-browser-access")).toBe("true");
    const content = body.messages[0].content;
    expect(content[0]).toMatchObject({ type: "document", source: { media_type: "application/pdf" } });
    expect(content.at(-1).text).toContain("Revenue up");
    expect(body.temperature).toBeUndefined();
  });

  it("skips server-side fallbacks for Haiku", async () => {
    const calls = mockApi({ title: "t", slides: [] });
    await generateDeck({ prompt: "x", source: "", slideCount: 3, tone: "", audience: "" }, { ...settings, model: "claude-haiku-5-5" });
    expect(calls[0].body.fallbacks).toBeUndefined();
  });

  it("explains refusals, truncation and bad keys in plain words", async () => {
    mockApi({ title: "t", slides: [] }, "refusal");
    await expect(generateDeck({ prompt: "x", source: "", slideCount: 3, tone: "", audience: "" }, settings)).rejects.toThrow(/declined/);
    mockApi({ title: "t", slides: [] }, "max_tokens");
    await expect(generateDeck({ prompt: "x", source: "", slideCount: 3, tone: "", audience: "" }, settings)).rejects.toThrow(/fewer slides/);
    mockApi({}, "end_turn", 401);
    await expect(generateDeck({ prompt: "x", source: "", slideCount: 3, tone: "", audience: "" }, settings)).rejects.toThrow(/key was rejected/);
  });

  it("refuses to call without a key", async () => {
    await expect(generateDeck({ prompt: "x", source: "", slideCount: 3, tone: "", audience: "" }, { apiKey: "", model: "" })).rejects.toBeInstanceOf(AIError);
  });
});

describe("editSlide", () => {
  it("keeps the slide id and a user-uploaded image", async () => {
    mockApi({ ...blankAiSlide, layout: "image", title: "Sharper", body: "New caption", imagePrompt: "x" });
    const slide = { ...starterSlide("image"), image: { url: "data:image/png;base64,AAA", prompt: "" } };
    const out = await editSlide(slide, "punchier", newDeck("d", [slide]), settings);
    expect(out.id).toBe(slide.id);
    expect(out.title).toBe("Sharper");
    expect(out.image.url).toBe("data:image/png;base64,AAA");
  });
});
