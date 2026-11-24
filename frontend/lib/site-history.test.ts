import { describe, expect, it } from "vitest"

import { hasPreviousPageInSite } from "@/lib/site-history"

const ORIGIN = "https://tabwallet.com.br"

describe("hasPreviousPageInSite", () => {
  describe("with the Navigation API", () => {
    it("is true when the browser has an earlier entry of the site", () => {
      expect(
        hasPreviousPageInSite({ navigation: { canGoBack: true }, referrer: "", origin: ORIGIN, navigatedInApp: false }),
      ).toBe(true)
    })

    it("is false on the first entry, even if the visitor came from another site", () => {
      expect(
        hasPreviousPageInSite({
          navigation: { canGoBack: false },
          referrer: "https://www.google.com/",
          origin: ORIGIN,
          navigatedInApp: false,
        }),
      ).toBe(false)
    })
  })

  describe("without it", () => {
    it("is false when the page was opened directly", () => {
      expect(hasPreviousPageInSite({ referrer: "", origin: ORIGIN, navigatedInApp: false })).toBe(false)
    })

    it("is false when the visitor came from another site", () => {
      expect(
        hasPreviousPageInSite({ referrer: "https://www.google.com/", origin: ORIGIN, navigatedInApp: false }),
      ).toBe(false)
    })

    it("is true after the visitor moved between pages of the site", () => {
      expect(hasPreviousPageInSite({ referrer: "", origin: ORIGIN, navigatedInApp: true })).toBe(true)
    })

    it("is true when the page was opened from another page of the site", () => {
      expect(
        hasPreviousPageInSite({ referrer: `${ORIGIN}/transacoes`, origin: ORIGIN, navigatedInApp: false }),
      ).toBe(true)
    })

    it("does not take a look-alike address for the site", () => {
      expect(
        hasPreviousPageInSite({ referrer: `${ORIGIN}.evil.example/`, origin: ORIGIN, navigatedInApp: false }),
      ).toBe(false)
    })
  })
})
