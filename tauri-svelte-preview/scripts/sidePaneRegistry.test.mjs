import assert from 'node:assert/strict';

import {
  assertValidSidePaneRegistrations,
  isValidSidePaneRegistrations,
  registrationsForRegion,
  sidePaneRegistrationToPaneSpec,
  sidePaneRegistrationsToPaneSpecs,
  validateSidePaneRegistrations
} from '../src/lib/shell/layout/sidePaneRegistry.ts';

const component = {};
const element = {};
const registration = (id, overrides = {}) => ({
  id,
  title: id,
  region: 'right',
  component,
  minimumSize: 28,
  preferredSize: 220,
  maximumSize: null,
  defaultExpanded: true,
  persistent: true,
  order: 0,
  ...overrides
});

// The contract accepts a valid roster and keeps region order deterministic.
{
  const roster = [
    registration('files', { order: 20 }),
    registration('source-control', { order: 10 }),
    registration('working', { region: 'left', order: 0 })
  ];
  assert.deepEqual(registrationsForRegion(roster, 'right').map((item) => item.id), [
    'source-control',
    'files'
  ]);
  assert.equal(validateSidePaneRegistrations(roster).valid, true);
  assert.equal(isValidSidePaneRegistrations(roster), true);
  assert.doesNotThrow(() => assertValidSidePaneRegistrations(roster));
}

// Duplicate ids/orders, invalid regions and broken size/persistence contracts
// are rejected before a PaneStack can be constructed.
{
  const invalid = [
    registration('same'),
    registration('same', { order: 1 }),
    registration('bad-region', { region: 'middle' }),
    registration('bad-size', { minimumSize: 300, preferredSize: 200, maximumSize: 100 }),
    registration('bad-persistence', { persistent: 'yes' })
  ];
  const result = validateSidePaneRegistrations(invalid);
  assert.equal(result.valid, false);
  assert.ok(result.errors.some((error) => error.includes('duplicate pane id')));
  assert.throws(() => assertValidSidePaneRegistrations(invalid), /Invalid side-pane registry/);
}

// Mapping is pure: it only translates registration fields into the existing
// PaneSpec shape and never mounts, stores, or calls a component.
{
  const item = registration('files', { defaultExpanded: false, persistent: false });
  const spec = sidePaneRegistrationToPaneSpec(item, element);
  assert.deepEqual(spec, {
    id: 'files',
    title: 'files',
    element,
    size: 220,
    expanded: false,
    minimumSize: 28,
    maximumSize: null,
    persistent: false
  });
  assert.deepEqual(
    sidePaneRegistrationsToPaneSpecs([item], new Map([['files', element]])).map((pane) => pane.id),
    ['files']
  );
  assert.throws(() => sidePaneRegistrationsToPaneSpecs([item], new Map()), /Missing DOM element/);
}

console.log('sidePaneRegistry: all tests passed');
