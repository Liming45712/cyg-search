export function toggleComparison(codes: string[], code: string): string[] {
  if (codes.includes(code)) return codes.filter((value) => value !== code);
  return codes.length < 2 ? [...codes, code] : codes;
}

export function officialRoleUrl(code: string): string | null {
  return /^[0-9]{1,32}$/.test(code)
    ? `http://cyg.changyou.com/details?goodsCode=${code}`
    : null;
}
