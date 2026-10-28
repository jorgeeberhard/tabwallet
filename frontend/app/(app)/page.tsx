import { Welcome } from "@/components/welcome"

export default function Home() {
  return (
    <section className="flex flex-col gap-2">
      <h1 className="font-display text-3xl font-bold tracking-tight">
        Seu dinheiro, em uma aba.
      </h1>
      <Welcome />
    </section>
  )
}
