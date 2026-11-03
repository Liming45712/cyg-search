import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Attr, Role } from './filter';
import { simulationFromSpirit } from './spirit-simulation';
import { calculateSpirit, spiritStats, treasureStats } from './spirit';
const attr = (raw:string,num:number|null=null):Attr=>({raw,num});
function sample():Role {
  return {goods_type:5,code:'1',name:'330g',title:'',job:'',price:100,server_id:'104',status:2,listed_at:'',offline_at:'',checked_at:0,detail_at:1,detail_error:null,
    detail:{equipment:{},attrs:{等级:attr('157',157),资质:attr('神工'),成长:attr('传说'),强化:attr('8',8),慧心:attr('5',5),
      力量资质:attr('2.73',2.73),体力资质:attr('2.77',2.77),心力资质:attr('2.43',2.43),耐力资质:attr('2.69',2.69),敏捷资质:attr('2.75',2.75),体质:attr('166',166)}}};
}
test('imports the selected detail snapshot, maps physique aptitude and carries the chosen star-general switch',()=>{
  const row = sample(); row.list_attrs={等级:attr('159',159),资质:attr('逆天')}; row.needs_refresh=true;
  const before=JSON.stringify(row);
  const simulation=simulationFromSpirit(row,false),input=simulation.input;
  assert.equal(input.level,157);assert.equal(input.quality,'神工');assert.equal(input.growth,'传说');assert.equal(input.stars,8);assert.equal(input.huixin,5);
  assert.equal(input.starGeneral,false);assert.equal(input.aptitude.体质,2.77);assert(simulation.stale);assert.deepEqual(simulation.missing,[]);
  assert.equal(input.insight,1);assert.equal(input.allocation,'无');assert.equal(input.huixinStat,'无');assert.equal(input.divineTreasure,false);
  for(const key of treasureStats){assert.equal(input.treasureBase[key],undefined);assert.equal(input.treasureExtra[key],undefined);}
  for(const key of spiritStats)assert.equal(input.treasureFive[key],undefined);
  assert.equal(calculateSpirit(input).error,null);
  input.aptitude.力量=99;assert.equal(JSON.stringify(row),before);
  assert.equal(simulationFromSpirit(row,true).input.aptitude.力量,2.73);
});
test('missing or invalid fields stay blank, and unparsed listings cannot simulate',()=>{
  const row=sample();row.detail!.attrs.资质=attr('未知');row.detail!.attrs.强化=attr('未知');row.detail!.attrs.慧心=attr('0',0);
  delete row.detail!.attrs.体力资质;
  const result=simulationFromSpirit(row,true);
  assert.equal(result.input.quality,'');assert.equal(result.input.stars,undefined);assert.equal(result.input.huixin,0);assert.equal(result.input.aptitude.体质,undefined);
  assert.deepEqual(result.missing,['资质品质','强化','体力资质']);
  row.detail=null;assert.throws(()=>simulationFromSpirit(row,true),/详情/);
});


test('defaults unavailable Huixin and insight to one while preserving a known zero',()=>{
  const row=sample();delete row.detail!.attrs.慧心;
  const result=simulationFromSpirit(row,true);
  assert.equal(result.input.huixin,1);assert.equal(result.input.insight,1);
  assert(!result.missing.includes('慧心'));assert.equal(calculateSpirit(result.input).error,null);
  row.detail!.attrs.慧心=attr('0',0);
  assert.equal(simulationFromSpirit(row,true).input.huixin,0);
});
