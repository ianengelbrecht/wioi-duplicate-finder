import { expect, test } from 'vitest'

import { parseElevation } from '../parseVerbatimElevation.js'

test('parseElevation returns nulls for empty input', () => {
  const result = parseElevation('')
  expect(result).toEqual({
    minElevation: null,
    maxElevation: null,
    elevation: null,
    elevationUncertainty: null,
    elevationUnit: null
  })
});

test('parseElevation correctly parses a range in feet', () => {
  const result = parseElevation('1000-2000 ft')
  expect(result).toEqual({
    minElevation: 305,
    maxElevation: 610,
    elevation: 458,
    elevationUncertainty: 153,
    elevationUnit: 'ft'
  })
});

test('parseElevation correctly parses a range in meters', () => {
  const result = parseElevation('300-400 m')
  expect(result).toEqual({
    minElevation: 300,
    maxElevation: 400,
    elevation: 350,
    elevationUncertainty: 50,
    elevationUnit: 'm'
  })
});

test('parseElevation correctly parses a single elevation in feet', () => {
  const result = parseElevation('1500 ft')
  expect(result).toEqual({
    minElevation: 458,
    maxElevation: 458,
    elevation: 458,
    elevationUncertainty: 0,
    elevationUnit: 'ft'
  })
});

test('parseElevation correctly parses a single elevation in meters', () => {
  const result = parseElevation('500 m')
  expect(result).toEqual({
    minElevation: 500,
    maxElevation: 500,
    elevation: 500,
    elevationUncertainty: 0,
    elevationUnit: 'm'
  })
});

test('parseElevation correctly parses a range with "to" in feet', () => {
  const result = parseElevation('1000 to 2000 ft')
  expect(result).toEqual({
    minElevation: 305,
    maxElevation: 610,
    elevation: 458,
    elevationUncertainty: 153,
    elevationUnit: 'ft'
  })
});

test('parseElevation correctly parses a range with "to" in meters', () => {
  const result = parseElevation('300 to 400 m')
  expect(result).toEqual({
    minElevation: 300,
    maxElevation: 400,
    elevation: 350,
    elevationUncertainty: 50,
    elevationUnit: 'm'
  })
});

test('parseElevation correctly parses a range with en dash in feet', () => {
  const result = parseElevation('1000–2000 ft')
  expect(result).toEqual({
    minElevation: 305,
    maxElevation: 610,
    elevation: 458,
    elevationUncertainty: 153,
    elevationUnit: 'ft'
  })
});

test('parseElevation correctly parses a range with en dash in meters', () => {
  const result = parseElevation('300–400 m')
  expect(result).toEqual({
    minElevation: 300,
    maxElevation: 400,
    elevation: 350,
    elevationUncertainty: 50,
    elevationUnit: 'm'
  })
});


