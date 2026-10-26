# Card

The container for one block of the dashboard or a page: a surface with a hairline border, `radius-lg` and `space-6` padding.

- Props: `title`, `subtitle`, `action` (usually a ghost `Button`), `flush` (no padding; for lists and tables that run edge to edge), children.
- One topic per card. Do not nest cards; separate content inside with `border` hairlines.
- Dashboard grid: one column on phones, two from `breakpoint-sm`, three from `breakpoint-lg`, with `space-4` gaps.

## shadcn/ui

`card` with `CardHeader`, `CardTitle`, `CardDescription`, `CardAction` (the `action` prop) and `CardContent`. `flush`: `CardContent` with `p-0` and a `separator` under the header.
