import { describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import path from 'node:path'
import vm from 'node:vm'
import {
  activePet,
  DEFAULT_PET,
  findPet,
  PACKS,
  PETS,
  petMarkup,
  seasonalPet,
} from '../../renderer/pets'

const renderer = path.join(import.meta.dir, '..', '..', 'renderer')
const css = fs.readFileSync(path.join(renderer, 'pets.css'), 'utf8')
const day = (y, m, d) => new Date(y, m - 1, d, 12)

describe('the pet catalog', () => {
  test('loads as a plain <script> too, as the widget does', () => {
    const page = {}
    vm.runInNewContext(fs.readFileSync(path.join(renderer, 'pets.js'), 'utf8'), page)
    expect(page.Pets.PETS.length).toBe(PETS.length)
  })

  test('every pet has a unique id, a known pack and a move the stylesheet defines', () => {
    const ids = PETS.map((p) => p.id)
    expect(new Set(ids).size).toBe(ids.length)
    expect(ids).not.toContain(DEFAULT_PET)
    for (const pet of PETS) {
      expect(PACKS.map((p) => p.id)).toContain(pet.pack)
      expect(css).toContain(`.pet-move-${pet.move} #claude`)
    }
  })

  test('recolours a pet for the moods the classic sprite recolours for', () => {
    for (const mood of ['stressed', 'tired', 'sleeping']) {
      expect(css).toMatch(new RegExp(`body\\.state-${mood} #skin \\{[^}]*filter:`))
    }
  })

  test('every seasonal pack has pets to pick from', () => {
    for (const pack of PACKS.filter((p) => p.days)) {
      expect(PETS.some((p) => p.pack === pack.id)).toBe(true)
    }
  })

  test('draws each pet with its own gradient ids, so two copies never collide', () => {
    for (const pet of PETS) {
      const a = petMarkup(pet.id, 'a-')
      const b = petMarkup(pet.id, 'b-', 'id="skin"')
      expect(a.startsWith('<svg ')).toBe(true)
      expect(b.startsWith('<svg id="skin" ')).toBe(true)
      const ids = (svg) => [...svg.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1])
      expect(ids(a).every((id) => id.startsWith('a-'))).toBe(true)
      expect(ids(a).filter((id) => ids(b).includes(id))).toEqual([])
    }
  })

  test('draws nothing for an unknown pet', () => {
    expect(petMarkup('nope', 'x')).toBe('')
    expect(petMarkup(DEFAULT_PET, 'x')).toBe('')
  })

  test('finds the classic sprite and the drawn pets, and nothing else', () => {
    expect(findPet(DEFAULT_PET).name).toBe('Clauddy')
    expect(findPet('wizard').pack).toBe('everyday')
    expect(findPet('nope')).toBeNull()
    expect(findPet(undefined)).toBeNull()
  })
})

describe('holidays', () => {
  const packOf = (id) => findPet(id).pack

  test('Halloween brings a Halloween pet on the 31st only', () => {
    expect(packOf(seasonalPet(day(2026, 10, 31)))).toBe('halloween')
    expect(seasonalPet(day(2026, 10, 30))).toBeNull()
    expect(seasonalPet(day(2026, 11, 1))).toBeNull()
  })

  test('Christmas keeps the same pet on the 24th and the 25th', () => {
    const eve = seasonalPet(day(2026, 12, 24))
    expect(packOf(eve)).toBe('christmas')
    expect(seasonalPet(day(2026, 12, 25))).toBe(eve)
    expect(seasonalPet(day(2026, 12, 26))).toBeNull()
  })

  test('the pick holds all day and changes from year to year', () => {
    expect(seasonalPet(new Date(2026, 9, 31, 0, 1))).toBe(
      seasonalPet(new Date(2026, 9, 31, 23, 59)),
    )
    const years = Array.from({ length: 12 }, (_, i) => seasonalPet(day(2026 + i, 10, 31)))
    expect(new Set(years).size).toBeGreaterThan(1)
  })

  test('a holiday overrides the saved pick, which comes back the next day', () => {
    expect(packOf(activePet('wizard', day(2026, 10, 31)))).toBe('halloween')
    expect(activePet('wizard', day(2026, 11, 1))).toBe('wizard')
  })

  test('an unknown or missing pick falls back to the classic sprite', () => {
    expect(activePet('gone', day(2026, 6, 1))).toBe(DEFAULT_PET)
    expect(activePet(undefined, day(2026, 6, 1))).toBe(DEFAULT_PET)
    expect(activePet(DEFAULT_PET, day(2026, 6, 1))).toBe(DEFAULT_PET)
  })
})
