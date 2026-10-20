version: "1"
package: lib.geometry@0.1.0
summary: 2D geometry primitives (Point, Rect) with union and containment helpers.

concepts:
  - id: value-geometry
    title: Value semantics
    body: Point and Rect are structs; copy freely without ARC overhead.

api:
  Lib.Geometry:
    Point:
      kind: struct
      blurb: Cartesian point with translate helper.
    Rect:
      kind: struct
      blurb: Axis-aligned rectangle from origin, width, and height.
    union:
      kind: func
      blurb: Minimal bounding rect covering two rects.

examples:
  - title: Area of a rect
    lang: ll
    code: |
      import Lib.Geometry
      let r = Rect(Point(0, 0), 10, 5)
      let a = r.area()

pitfalls:
  - Rect uses origin + size, not center + size.
  - union assumes axis-aligned inputs.

conventions:
  - Namespace Lib.Geometry matches package id lib.geometry.

tokens:
  priority: normal
  prefer: [summary, api, examples]
