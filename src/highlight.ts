export function highlightParts(text: string, keyword: string) {
  const query = keyword.trim();
  if (!query) return [{ text, match: false }];
  return text.split(query).flatMap((part, index) => index
    ? [{ text: query, match: true }, { text: part, match: false }]
    : [{ text: part, match: false }]);
}
