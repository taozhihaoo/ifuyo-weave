import { describe, expect, it } from "vitest";
import { suggestionsFor } from "./quickDrop";

describe("suggestionsFor (§25-§33 Quick Drop)", () => {
  it("maps image extensions to image view", () => {
    expect(suggestionsFor("file", "png")).toEqual(["image", "documents"]);
    expect(suggestionsFor("file", "jpg")).toEqual(["image"]);
  });

  it("maps document extensions to documents view", () => {
    expect(suggestionsFor("file", "pdf")).toEqual(["documents"]);
    expect(suggestionsFor("file", "docx")).toEqual(["documents"]);
  });

  it("maps data extensions to data view", () => {
    expect(suggestionsFor("file", "csv")).toEqual(["data"]);
    expect(suggestionsFor("file", "xlsx")).toEqual(["data", "documents"]);
  });

  it("maps text extensions to text view", () => {
    expect(suggestionsFor("file", "txt")).toEqual(["text"]);
    expect(suggestionsFor("file", "md")).toEqual(["text"]);
  });

  it("directory suggests duplicates + workflow (§30)", () => {
    expect(suggestionsFor("directory", "")).toEqual(["duplicates", "workflow"]);
  });

  it("unknown extension suggests nothing (§26 不自作聪明)", () => {
    expect(suggestionsFor("file", "exe")).toEqual([]);
    expect(suggestionsFor("file", "")).toEqual([]);
  });

  it("case-insensitive extension", () => {
    expect(suggestionsFor("file", "PNG")).toEqual(["image", "documents"]);
  });
});
