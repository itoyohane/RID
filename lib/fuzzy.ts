import type { AppInfo } from "@/lib/types";

export function normalizeSearch(value: string) {
  return value.toLocaleLowerCase().replace(/[\s._/\\():-]+/g, "");
}

function subsequenceScore(text: string, query: string) {
  let queryIndex = 0;
  let score = 0;
  let lastMatch = -1;

  for (let index = 0; index < text.length && queryIndex < query.length; index += 1) {
    if (text[index] === query[queryIndex]) {
      score += lastMatch < 0 ? index : index - lastMatch - 1;
      lastMatch = index;
      queryIndex += 1;
    }
  }

  return queryIndex === query.length
    ? score + text.length * 0.01
    : Number.POSITIVE_INFINITY;
}

export function scoreApp(app: AppInfo, rawQuery: string) {
  const query = normalizeSearch(rawQuery);
  if (!query) return 0;

  const candidates = [app.name, app.path, ...app.aliases].map(normalizeSearch);
  return Math.min(
    ...candidates.map((candidate) => {
      const exactIndex = candidate.indexOf(query);
      if (exactIndex >= 0) return exactIndex * 0.1;
      return 10 + subsequenceScore(candidate, query);
    }),
  );
}

