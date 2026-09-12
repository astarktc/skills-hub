import { describe, expect, it } from "vitest";
import { parseFrontmatter } from "./manifestPresentation";
import fenceCorpus from "./manifestPresentation.corpus.json";

describe("manifest presentation", () => {
  it.each(fenceCorpus)(
    "complete column-zero fences: $label",
    ({ raw, meta, body }) => {
      expect(parseFrontmatter(raw)).toEqual({ meta, body });
    },
  );

  it("preserves existing metadata and body presentation", () => {
    expect(parseFrontmatter(
      '---\nname: "alpha"\ndescription: |\n  before\n  ---\n  after\nuser-invocable: false\n---\n\nBody\n---\nlater\n',
    )).toEqual({
      meta: {
        name: "alpha",
        description: "before\n---\nafter",
        "user-invocable": "false",
      },
      body: "Body\n---\nlater\n",
    });
    expect(parseFrontmatter(
      "---\nname: alpha\ndescription: >\n  one\n  two\n---\nBody",
    )).toEqual({
      meta: { name: "alpha", description: "one two" },
      body: "Body",
    });
  });

  it("keeps absent, empty and unfinished frontmatter as body", () => {
    for (const raw of ["", "Body\n", "---\n---\nBody", "---\nname: unfinished\n"]) {
      expect(parseFrontmatter(raw)).toEqual({ meta: null, body: raw });
    }
  });
});
