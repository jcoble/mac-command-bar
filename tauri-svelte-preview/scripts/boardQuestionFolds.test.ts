import assert from 'node:assert/strict';

import { FOLDS_KEY, isFoldOpen, readFolds, writeFolds } from '../src/lib/shell/board/boardQuestionFolds.ts';

const memory = (): Storage => {
  const data = new Map<string, string>();
  return {
    get length() { return data.size; },
    clear: () => data.clear(),
    getItem: (key) => data.get(key) ?? null,
    key: (index) => [...data.keys()][index] ?? null,
    removeItem: (key) => void data.delete(key),
    setItem: (key, value) => void data.set(key, value)
  };
};

{ // an_open_question_starts_expanded_and_an_answered_one_collapsed
  const folds = readFolds(memory());
  assert.equal(isFoldOpen(folds, 'mac:1', false), true);
  assert.equal(isFoldOpen(folds, 'mac:2', true), false);
}
{ // a_stored_fold_wins_over_the_default
  const storage = memory();
  writeFolds(storage, { 'mac:1': false, 'mac:2': true }, ['mac:1', 'mac:2'], ['mac']);
  const folds = readFolds(storage);
  assert.equal(isFoldOpen(folds, 'mac:1', false), false);
  assert.equal(isFoldOpen(folds, 'mac:2', true), true);
}
{ // ids_no_longer_on_the_board_are_dropped_on_write
  const storage = memory();
  writeFolds(storage, { 'mac:1': false, 'mac:9': false }, ['mac:1'], ['mac']);
  assert.deepEqual(JSON.parse(storage.getItem(FOLDS_KEY) ?? ''), { 'mac:1': false });
}
{ // an_unreachable_machine_keeps_its_folds
  const storage = memory();
  writeFolds(storage, { 'mac:1': false, 'box:4': false }, ['mac:1'], ['mac']);
  assert.deepEqual(JSON.parse(storage.getItem(FOLDS_KEY) ?? ''), { 'mac:1': false, 'box:4': false });
}
{ // a_damaged_value_reads_as_no_folds
  const storage = memory();
  storage.setItem(FOLDS_KEY, 'not json');
  assert.deepEqual(readFolds(storage), {});
}
console.log('board question fold tests passed');
