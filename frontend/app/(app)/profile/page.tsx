import type { Metadata } from "next"

import { ProfileForms } from "@/components/profile/profile-forms"

export const metadata: Metadata = { title: "Perfil | TabWallet" }

export default function ProfilePage() {
  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h1 className="font-display text-3xl font-bold tracking-tight">Perfil</h1>
        <p className="text-muted-foreground">Seus dados de acesso ao TabWallet.</p>
      </div>
      <ProfileForms />
    </section>
  )
}
