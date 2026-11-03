import { test } from 'node:test';
import assert from 'node:assert/strict';
import { highlightParts } from './highlight';

test('highlights every literal occurrence and preserves all original text', () => {
  for (const [text, query, hits] of [['攻击力+40，攻击力+40',' 攻击力 ',2],['SP+5% / SP+5%','SP+5%',2],['套装[1].*套装','[1].*',1],['<img src=x onerror=alert(1)>','<img',1],['凤翼','攻击',0],['HP','hp',0],['暴击率','   ',0]] as const) {
    const parts = highlightParts(text,query);
    assert.equal(parts.map(part=>part.text).join(''),text);
    assert.equal(parts.filter(part=>part.match).length,hits);
    assert(parts.filter(part=>part.match).every(part=>part.text===query.trim()));
  }
});
