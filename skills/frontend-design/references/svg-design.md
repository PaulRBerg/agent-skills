# SVG Design

Use original SVG to give a page identity, depth, and visual rhythm. Draw shapes that belong to its subject and
composition. A useful motif can turn an empty header into a recognizable product surface without adding more copy.

## Choose a visual idea

Start with the page's subject, not an SVG effect. Translate one domain relationship into geometry. Use these examples as
starting points, not assets to repeat across unrelated products:

| Subject                      | Visual idea                                         | Useful placement                      |
| ---------------------------- | --------------------------------------------------- | ------------------------------------- |
| Coordination and workflows   | Separate paths join at a shared node                | Header edge or section transition     |
| Mapping and exploration      | Contours bend around a focal region                 | Background beside an introduction     |
| Sound and communication      | Sparse waveforms or interference arcs               | Media header or editorial margin      |
| Architecture and fabrication | Offset planes, section lines, or construction marks | Layout frame or product detail        |
| Research and discovery       | An orbit, specimen outline, or measured curve       | Focal illustration or chapter opening |

Choose one geometry family and repeat its rules. Extend a curve into a divider, or reuse a node shape in a small accent.
Use a different crop or scale for each placement. Do not stamp the same illustration onto every card.

Distinguish illustration from evidence. Decorative routes must not appear to show live connections. Invented contours,
waveforms, or curves must not imply measured data. If a graphic carries real information, use accurate data and give it
an accessible explanation.

## Compose before adding effects

1. Reserve quiet space for the title, controls, and important values. Place the densest geometry beyond that area.
2. Choose a focal point and a direction of travel. Let curves approach a heading, connect sections, or frame a feature.
3. Establish scale contrast. Combine one broad gesture with a few precise details, such as a fine line and a filled
   node.
4. Use deliberate asymmetry and cropping. Let a path enter from an edge instead of enclosing every composition in a box.
5. Separate depth into a broad shape, a supporting line, and a restrained accent. Add only the layers the composition
   needs.
6. Tune color against the actual surface. Start with a muted surface-relative color and one accent from the product
   palette.

Draw the silhouette first. Check whether it remains recognizable without gradients, blur, or animation. Use Bézier
control points to make curves share a clear direction and smooth joins. Align endpoints and intersections deliberately.
Let spacing, line weight, and repetition create rhythm before adding detail.

Gradients can fade a motif into its surface or direct attention toward a node. Keep stops related to the palette and
match their direction to the geometry. Avoid using a multicolor glow as a substitute for composition. Compare light and
dark treatments separately because the same opacity produces different results on each surface.

## Place it in the layout

- **Background:** Anchor the motif to the section it supports. Leave a clear region behind text and controls.
- **Divider:** Use a contour, stepped edge, or connecting path to carry the eye between sections. Preserve reading
  order.
- **Frame:** Place partial borders or corner details around meaningful content. Keep their contrast below interactive
  states.
- **Focal illustration:** Give one graphic enough space to explain the domain idea. Let nearby typography remain simple.
- **Small accent:** Use a fragment near an empty state or section label. Keep it distinct from buttons and status
  indicators.

Choose the crop at narrow widths rather than shrinking the entire desktop composition. Move, simplify, or hide secondary
geometry when it crowds content. Use normal document flow for a graphic that needs space of its own.

## Build with code-native SVG

- Use paths, circles, lines, and simple fills directly. Prefer CSS for effects that do not need vector geometry.
- Keep a readable `viewBox` coordinate system. Choose `preserveAspectRatio` deliberately: `meet` contains the drawing,
  while `slice` fills the box and may crop it. Avoid `none` unless distortion is intentional.
- Use inline SVG when the graphic needs inherited theme colors or component state. Use an SVG asset when it is static.
  An SVG loaded through an image or CSS URL does not inherit the page's custom properties.
- Reuse existing color tokens through `currentColor` or CSS custom properties for inline SVG. Keep theme decisions in
  the project's styling system.
- Give decorative SVG `aria-hidden="true"` and `focusable="false"`. Keep it free of links and focusable children. Give
  meaningful graphics an accessible name or equivalent nearby content instead.
- Set `pointer-events: none` on decoration. Keep controls above it and preserve visible focus indicators.
- Contain absolute decoration in a positioned wrapper. Clip its decorative layer without clipping menus or focus rings.
  Keep decoration out of layout sizing so it cannot create horizontal scrolling.
- Give IDs for gradients, masks, and clip paths a unique value per component instance. Update their fragment references
  together. For repeated React components, use the project's stable ID mechanism, such as `useId`.
- Use few nodes and simple geometry. Add filters only when they make a visible improvement at the delivered size. Large
  blur and turbulence regions can be expensive to paint.
- Keep the first version static. If motion supports the page, animate sparingly and provide a static reduced-motion
  state.

## Example: paths converging at a shared node

This decorative header motif leaves the left side open for content. Adapt its geometry, crop, and colors to the actual
layout. Do not treat the example as a default background for every site.

```html
<svg
  xmlns="http://www.w3.org/2000/svg"
  class="coordination-motif"
  viewBox="0 0 960 240"
  preserveAspectRatio="xMaxYMid slice"
  aria-hidden="true"
  focusable="false"
>
  <g fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
    <path d="M430 -20 C610 -20 570 120 740 120 S880 70 1000 70" opacity=".28" />
    <path d="M480 260 C620 260 600 120 740 120 S900 190 1000 190" opacity=".2" />
    <path d="M540 120 H1000" opacity=".12" />
    <circle cx="740" cy="120" r="22" opacity=".16" />
  </g>
  <circle cx="740" cy="120" r="4" fill="currentColor" opacity=".6" />
</svg>
```

Place this SVG inside an absolutely positioned decorative layer that fills its section. Apply these rules through the
project's styling system. Supply `--motif-color` from a theme token chosen for that surface.

```css
.coordination-motif {
  display: block;
  width: 100%;
  height: 100%;
  color: var(--motif-color);
  pointer-events: none;
}
```

The right-aligned crop suits a wide header with text on the left. At narrow widths, choose a quieter fragment or hide
the motif. Do not let this crop move the shared node behind the title.

## Review the rendered result

Inspect the SVG in the page, with real content, at the viewports and themes required by the main workflow.

- **Identity:** Does the geometry express this subject? Revise motifs that pass unchanged into unrelated products.
- **Hierarchy:** Does content remain dominant? Inspect the area where decoration is brightest or most detailed.
- **Composition:** Are curves, crops, intersections, and empty space deliberate? Remove accidental tangencies and
  clutter.
- **Interaction:** Can users still click, select text, and see keyboard focus? Decoration must not imply unavailable
  controls.
- **Responsive behavior:** Is the chosen crop intentional? Check overflow, overlap, and the longest expected heading.
- **Delivery:** Do colors, fragment references, repeated instances, and reduced-motion behavior work in the rendered
  page?

Compare the page with the motif visible and hidden. Keep it when it adds identity or improves composition without
obscuring the page's job. Simplify or remove it when it only adds noise.

## Technical references

- [SVG viewBox](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/viewBox)
- [SVG preserveAspectRatio](https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Attribute/preserveAspectRatio)
- [CSS pointer-events](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/pointer-events)
