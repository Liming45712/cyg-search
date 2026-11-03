import type { Role } from './filter';

export function currencyQuote(row: Role) {
  // Currency quantity is in the explicitly labelled product name; API quantity=1 means one listing.
  const match = /^银两\s*[x×]\s*((?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?)\s*(亿|万)(?:银两)?\s*$/i.exec(row.name.trim());
  const amount = match ? Number(match[1].replaceAll(',', '')) * (match[2] === '亿' ? 100_000_000 : 10_000) : NaN;
  const coins = Number.isFinite(amount) && amount > 0 && amount <= Number.MAX_SAFE_INTEGER ? amount : null;
  const price = row.price !== null && Number.isFinite(row.price) && row.price > 0 ? row.price : null;
  const ratio = coins !== null && price !== null ? coins / price : null;
  const coinsPerYuan = ratio !== null && Number.isFinite(ratio) ? ratio : null;
  return { ...row, coins, yi: coins === null ? null : coins / 100_000_000, coinsPerYuan };
}
