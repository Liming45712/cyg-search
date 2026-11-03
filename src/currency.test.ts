import { test } from 'node:test';
import assert from 'node:assert/strict';
import { currencyQuote } from './currency';
import type { Role } from './filter';
const row = (name: string, price: number | null) => ({name,price} as Role);
test('official 2.6亿 / 96元 is 270.8333万 per yuan, not 2.6 coins or the rounded inverse unit price', () => {
  const quote = currencyQuote(row('银两 x2.6亿',96));
  assert.equal(quote.coins,260_000_000);
  assert.equal(quote.yi,2.6);
  assert.equal(quote.coinsPerYuan,260_000_000/96);
  assert.equal((quote.coinsPerYuan!/10_000).toFixed(4),'270.8333');
  assert.equal(currencyQuote(row('银两 ×26000万',96)).coins,quote.coins);
  assert.equal(currencyQuote(row('银两 x1亿',100)).coinsPerYuan,1_000_000);
});
test('unknown units, missing values and zero prices never produce a misleading rate', () => {
  for (const name of ['银两 x2.6','银两 x2.6万亿','银两 x0亿','银两 x-1亿','银两 x未知亿','银两 x1,,2亿']) assert.equal(currencyQuote(row(name,96)).coinsPerYuan,null);
  for (const price of [0,-1,null,Infinity,NaN]) assert.equal(currencyQuote(row('银两 x2.6亿',price)).coinsPerYuan,null);
});
