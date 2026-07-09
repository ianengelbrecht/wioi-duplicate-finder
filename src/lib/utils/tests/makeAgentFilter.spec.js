import { expect, test } from 'vitest'

import { makeAgentFilter } from '../makeAgentFilter.js'

const testCases = [
  "Jones",
  "Jones, A.B",
  "Jones, A.B.",
  "Jones, AB",

  "Van der Merwe",
  "Van der Merwe, A.B.",
  "Van der Merwe, AB",

  "School of Biology",

  "A.B Jones",
  "Jones AB",
  "AB Jones"
]

test('makeAgentFilter correctly filters names if we want initials with periods', () => {
  const nameFilter = makeAgentFilter({ initialsRequirePeriods: true });
  const results = testCases.filter(nameFilter);

  expect(results).toEqual([
    "Jones",
    "Jones, A.B.",
    "Van der Merwe",
    "Van der Merwe, A.B.",
    "School of Biology"
  ]);
});

test('makeAgentFilter correctly filters names if we do not want initials with periods', () => {
  const nameFilter = makeAgentFilter({ initialsRequirePeriods: false });
  const results = testCases.filter(nameFilter);

  expect(results).toEqual([
    "Jones",
    "Jones, AB",
    "Van der Merwe",
    "Van der Merwe, AB",
    "School of Biology"
  ]);
});