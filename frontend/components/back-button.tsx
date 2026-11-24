"use client"

import { ArrowLeftIcon } from "lucide-react"
import { usePathname, useRouter } from "next/navigation"
import { useEffect, useRef } from "react"

import { Button } from "@/components/ui/button"
import { hasPreviousPageInSite } from "@/lib/site-history"

/**
 * "Voltar" below the header of every inner page (not the home). Goes back in
 * the browser history while the site has an earlier page there, and to the home
 * page otherwise (a bookmark, a link from outside, a page opened in a new tab).
 *
 * The layout keeps it mounted across pages, so it also notices when the visitor
 * moves between pages, for browsers without the Navigation API.
 */
export function BackButton() {
  const pathname = usePathname()
  const router = useRouter()
  const lastPathname = useRef(pathname)
  const navigatedInApp = useRef(false)

  useEffect(() => {
    if (lastPathname.current !== pathname) {
      lastPathname.current = pathname
      navigatedInApp.current = true
    }
  }, [pathname])

  if (pathname === "/") return null

  function goBack() {
    const previous = hasPreviousPageInSite({
      navigation: (window as { navigation?: { canGoBack: boolean } }).navigation,
      referrer: document.referrer,
      origin: window.location.origin,
      navigatedInApp: navigatedInApp.current,
    })
    if (previous) router.back()
    else router.push("/")
  }

  return (
    <div className="mx-auto w-full max-w-[1200px] px-4 pt-4 md:px-6">
      <Button variant="ghost" onClick={goBack} className="-ml-3">
        <ArrowLeftIcon strokeWidth={1.75} data-icon="inline-start" />
        Voltar
      </Button>
    </div>
  )
}
