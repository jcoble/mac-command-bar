import assert from 'node:assert/strict';

import { placeSessionContextMenu } from '../src/lib/shell/sessionLibrary/sessionLibraryContextMenu.ts';

const viewport = { width: 1_440, height: 900 };
const menu = { width: 224, height: 250 };

assert.deepEqual(
  placeSessionContextMenu(
    {
      left: 1_170,
      right: 1_198,
      top: 250,
      bottom: 278,
      containingBlockLeft: 300,
      containingBlockRight: 1_200,
      containingBlockTop: 80,
      containingBlockBottom: 820
    },
    menu,
    viewport
  ),
  { left: 638, top: 206 },
  'a right-edge action flips left and remains inside its Dockview containing block'
);

assert.deepEqual(
  placeSessionContextMenu(
    { left: 40, right: 68, top: 780, bottom: 808 },
    menu,
    viewport
  ),
  { left: 76, top: 522 },
  'an unconstrained action opens right and flips above the viewport bottom'
);

console.log('session context menu placement tests passed');
