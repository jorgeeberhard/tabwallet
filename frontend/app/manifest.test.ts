import { readFileSync } from "node:fs"
import { join } from "node:path"
import { describe, expect, it } from "vitest"

import manifest from "@/app/manifest"

/** Width and height from the IHDR chunk of a PNG file. */
function pngSize(path: string): string {
  const header = readFileSync(path)
  return `${header.readUInt32BE(16)}x${header.readUInt32BE(20)}`
}

describe("web manifest", () => {
  it("declares the 192 and 512 px icons and the files really have those sizes", () => {
    const icons = manifest().icons ?? []

    expect(icons.map((icon) => icon.sizes)).toEqual(["192x192", "512x512"])
    for (const icon of icons) {
      expect(pngSize(join(process.cwd(), "public", icon.src))).toBe(icon.sizes)
    }
  })

  it("has the apple-touch-icon at 180 px", () => {
    expect(pngSize(join(process.cwd(), "app", "apple-icon.png"))).toBe("180x180")
  })

  it("opens the app on the home page, in Portuguese", () => {
    expect(manifest()).toMatchObject({ name: "TabWallet", start_url: "/", lang: "pt-BR" })
  })
})
