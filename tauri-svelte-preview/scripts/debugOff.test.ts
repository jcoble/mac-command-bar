/**
 * debugOff.test.ts — reading ASSEMBLY_DEBUG_OFF into the set of chat parts that are switched off.
 *
 * Run: node --experimental-strip-types scripts/debugOff.test.ts
 */
import assert from 'node:assert/strict';
import { DEBUG_OFF_NAMES, parseDebugOff, parseDebugSessions } from '../src/lib/shell/conversation/debugOff.ts';

const parsed = (value: string) => [...parseDebugOff(value)].sort();

// nothing_set_switches_nothing_off
assert.deepEqual(parsed(''), []);
assert.deepEqual(parsed('none'), []);

// a_comma_list_switches_those_parts_off
assert.deepEqual(parsed('tools,markdown'), ['markdown', 'tools']);
assert.deepEqual(parsed(' Tool-Output , code ,,'), ['code', 'tool-output']);

// all_switches_every_part_off
assert.deepEqual(parsed('all'), [...DEBUG_OFF_NAMES].sort());
assert.equal(DEBUG_OFF_NAMES.length, 8);

// an_unknown_name_is_ignored
assert.deepEqual(parsed('tools,spinners'), ['tools']);

// the_switch_sessions_are_a_comma_list_of_ids
assert.deepEqual(parseDebugSessions(''), []);
assert.deepEqual(parseDebugSessions(' a1, b2 ,,'), ['a1', 'b2']);

console.log('debugOff: all checks passed');
