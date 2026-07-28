/**
 * Covers the two names behind the editor's "N references" margin number: the
 * lens id Monaco identifies a lens by (and hands back on click), and the key
 * the editor remembers a counted number under. The rules live in
 * `src/lib/sourceCodeLensKeys.ts` so they can be checked without Monaco.
 */
import assert from 'node:assert/strict';
import {
	sourceCodeLensCountKey,
	sourceCodeLensId,
	sourceCodeLensSpotFromId
} from '../src/lib/sourceCodeLensKeys.ts';

const spot = (symbolName, line, column) => ({ symbolName, line, column });

// an id carries the spot back intact when the user clicks the number
{
	const original = spot('RunAsync', 42, 17);
	assert.deepEqual(sourceCodeLensSpotFromId(sourceCodeLensId(original)), original);
}

// a symbol name with a colon in it does not split the id
{
	const original = spot('System::Widget', 3, 9);
	assert.deepEqual(sourceCodeLensSpotFromId(sourceCodeLensId(original)), original);
}

// the id is made of the spot and nothing else — no model version, which is
// what used to give every lens a new identity on every keystroke
{
	assert.equal(sourceCodeLensId(spot('RunAsync', 42, 17)), 'mcb-ref-count:42:17:RunAsync');
}

// a symbol that moved to another line is a different lens
{
	assert.notEqual(sourceCodeLensId(spot('RunAsync', 42, 17)), sourceCodeLensId(spot('RunAsync', 43, 17)));
}

// somebody else's lens, and damaged ids, are not ours to read
{
	assert.equal(sourceCodeLensSpotFromId(undefined), null);
	assert.equal(sourceCodeLensSpotFromId('some-other-provider:1:2:Alpha'), null);
	assert.equal(sourceCodeLensSpotFromId('mcb-ref-count:1:2:'), null, 'no symbol name');
	assert.equal(sourceCodeLensSpotFromId('mcb-ref-count:x:2:Alpha'), null, 'line is not a number');
	assert.equal(sourceCodeLensSpotFromId('mcb-ref-count:1:x:Alpha'), null, 'column is not a number');
}

// a remembered number is named by file, line and symbol
{
	const key = sourceCodeLensCountKey('file:///project/App.cs', spot('RunAsync', 42, 17));
	assert.equal(key, 'file:///project/App.cs::42:RunAsync');
}

// text typed earlier on the same line does not throw the number away
{
	assert.equal(
		sourceCodeLensCountKey('file:///project/App.cs', spot('RunAsync', 42, 17)),
		sourceCodeLensCountKey('file:///project/App.cs', spot('RunAsync', 42, 25))
	);
}

// the same symbol in another file is counted under its own name
{
	assert.notEqual(
		sourceCodeLensCountKey('file:///project/App.cs', spot('RunAsync', 42, 17)),
		sourceCodeLensCountKey('file:///project/Other.cs', spot('RunAsync', 42, 17))
	);
}

console.log('sourceCodeLensKeys tests passed');
