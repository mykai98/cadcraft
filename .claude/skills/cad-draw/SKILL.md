---
name: cad-draw
description: Draw in the running CADCraft app through the cadcraft MCP tools — plan the geometry, drive the command line, verify every step with query_entities, and show the result with render. Use whenever the user asks to draw, sketch, dimension, hatch, or edit a drawing ("画图", "画一个…", "draw a …").
---

# Drawing with the CADCraft MCP tools

CADCraft is an AutoCAD-parity CAD app. Coordinates are drawing units, **y up**. The MCP server bridges to the live app; every mutation is immediately visible to the user, so verify as you go.

## Workflow

1. **Start from a known state.** `mcp__cadcraft__inspect_drawing` first. For a fresh drawing use `mcp__cadcraft__new_drawing`; never wipe the user's open drawing without being asked.
2. **Plan before drawing.** Compute the key coordinates yourself (centers, tangent points, vertices) — don't eyeball them. Prefer absolute coordinates; use relative (`@dx,dy`) and polar (`@dist<angle`) input for chained segments.
3. **Draw with `mcp__cadcraft__command_line`**, one command per call, AutoCAD syntax:
   - `line 0,0 100,0 100,50 c` (`c` closes), `pline` for a single polyline entity
   - `circle 50,25 10` (center radius), `arc` (3 points), `ellipse`, `rectang 0,0 100,50`, `polygon`
   - Edits: `offset 5` then pick, `trim`, `extend`, `fillet` (set `r` first), `chamfer`, `mirror`, `move`, `copy`, `rotate`, `scale`, `array`
   - Annotation: `text`, `mtext`, `dimlinear`/`dimaligned`/`dimradius`, `hatch`, `bhatch`
   - If a command prompts for more input, answer in the next `command_line` call; `mcp__cadcraft__cancel` aborts a stuck prompt.
4. **Prefer `mcp__cadcraft__execute` (JSON) for precision** — exact parameters, defaults filled in, never opens a dialog. `mcp__cadcraft__list_commands` shows ids and params when unsure. For many steps at once use `mcp__cadcraft__script`.
5. **Verify after every mutating step**: `mcp__cadcraft__query_entities` (type, layer, geometry) or `inspect_drawing` (counts, extents). Geometry truth comes from queries, **never** from looking at a screenshot. If a step produced the wrong entity, `u` (undo) and redo it correctly.
6. **Show the result**: `command_line` `zoom e`, then `mcp__cadcraft__render` (drawing only) or `mcp__cadcraft__screenshot` (whole UI). Check the image yourself before telling the user it's done.
7. **Save only when asked** (`mcp__cadcraft__save`).

## Conventions

- **Layers**: structure drawings like a drafter — `layer` command (`make <name>`, set color/linetype) before drawing a group; dims on a dim layer, hatches on a hatch layer.
- **Units/angles**: angles are degrees counter-clockwise, 0 = east. Distances are drawing units.
- **Snaps off for scripted input**: typed coordinates are exact; don't rely on osnap. If a pick-based edit (trim/offset side) misbehaves, prefer the JSON `execute` form with explicit points.
- **Undo is cheap**: `u` reverts the last command — fix mistakes immediately rather than drawing over them.
- **Selection in commands**: pick points go through `command_line` as coordinates; `all` selects everything when a command asks "Select objects".

## Example: flanged plate

```
new_drawing
command_line: rectang 0,0 120,80
command_line: fillet r 10          → then: fillet, pick two edges … (or execute fillet with JSON)
command_line: circle 15,15 5
command_line: array …              (3 more corner holes; or copy with @dx,dy)
query_entities                     → expect 1 polyline + 4 circles
command_line: zoom e
render                             → confirm visually, report to the user
```
