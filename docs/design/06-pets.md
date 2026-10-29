# Pets

The pets Clauddy can wear besides the classic sprite: the catalog and its packs, how a pet is drawn and moves, the holiday rule, and how to add one.

## Overview

**⚙ Settings → Appearance → pet** shows the current pet and **Change ›**, which opens the catalog. A pick previews at once; **Save** keeps it as `config.pet`, **Cancel** puts the saved one back. An unknown id falls back to the classic sprite. On holidays a pet from the season's pack takes over for the day, whatever was picked.

| Pack | Pets | Its days |
|---|---|---|
| Everyday | Clauddy (the classic sprite), Wizard | — |
| Halloween | Pumpkin, Skull, Ghost, Bat, Black cat | October 31 |
| Christmas | Snowman, Santa, Reindeer, Gingerbread | December 24 and 25 |

## How a pet is drawn

Every pet except the classic sprite is vector SVG written as a function in `renderer/pets.js`, with its animations in `renderer/pets.css`. All ten weigh about 50 KB, so they ship inside the app and nothing is fetched at runtime ([ADR-008](../decisions/ADR-008-pets-drawn-in-code.md)).

- Each builder takes a **prefix** for its gradient ids, so the widget and the Settings thumbnails can draw the same pet at once without their ids colliding.
- The chosen pet is drawn **inside `#claude`** as `#skin`, so every mood animation on `#claude` (sleeping, slump, poke, celebrate, the shiver on fire) keeps working.
- Its parts animate **only under `#skin`**, so the thumbnails in Settings stay still.
- Every keyframe carries a `pk-` prefix, so none collides with the widget's own.

## Moving and moods

| | Classic sprite | A pet |
|---|---|---|
| Idle | Breathes and blinks | Still, blinks |
| Working | The shared hop | Its own move (`pet-move-*`: rock, dance, drift, wiggle, sway, jolly, prance, jump, levitate) and its own details: the Bat flaps, the Wizard casts while an owl flies by, snow falls around the Snowman |
| On fire | Turns red | A warm filter, flames rising behind it |
| Maxed out | Drained colour | Grey filter, eyes half closed |
| Sleeping | Darker | Dimmed, eyes closed |

The eyes follow the cursor and glance toward a provider, as the sprite's do: the pet's blink groups move with `translate`, which composes with the blink's own transform. Next to two or more provider rows a pet is fitted into the sprite's own box, so it does not reach the account row. In pet-only mode the usage bubble is placed once per show, just over the pet's head, measured at its resting spot so the animation does not move it.

## Holidays

On a pack's days, one of its pets takes over ([ADR-009](../decisions/ADR-009-holiday-pet-by-hash.md)):

- **One per pack and year**, chosen by a hash of the year and the pack, so the pick holds all day and across restarts without being stored, and changes from year to year.
- The widget checks every minute, so the switch happens at midnight with it open.
- The saved pick is untouched and comes back the next day. Settings says when a holiday pet is visiting.

## Adding a pet

1. Add an entry to `PETS` in `renderer/pets.js`: `id`, `name`, `pack`, `move` and `svg: (p) => ...`, with every gradient id prefixed with `${p}` and the root `<svg ROOT viewBox="...">`.
2. Wrap the eyes in `<g class="blink">`, put what shows only while working under `only-work` and what shows only when idle under `only-idle`.
3. Add its part animations to `renderer/pets.css` under `body.state-working #skin`, with `pk-` keyframes. A new `move` needs its `body.state-working.pet-move-<move> #claude` rule.
4. A new pack goes in `PACKS`, with `days` if it should take over on holidays.
5. `bun run test` checks every pet: a unique id, a known pack, a move the stylesheet defines, gradient ids that never collide.

## Still open

- [ ] **Theme colour.** Choosing the widget's colours was the other half of the original request; it is tracked in [#67](https://github.com/renatoaug/claude-usage-monitor/issues/67).
- [ ] **The README video and the GIFs show only the classic sprite.** The pets appear only in the video attached to [#66](https://github.com/renatoaug/claude-usage-monitor/pull/66).
