import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useServerFn } from "@tanstack/react-start";
import { CheckCircle2, Github, Heart, Loader2, Mail, MessageSquare } from "lucide-react";

import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { submitContactForm } from "@/lib/contact.functions";
import { content, type Language } from "@/lib/i18n";

export const Route = createFileRoute("/kontakt")({
  head: () => ({
    meta: [
      {
        title: "Kontakt | FileIT — Inteligentní správa dokumentů | file-app.uk",
      },
      {
        name: "description",
        content:
          "Máte dotaz, nápad nebo potřebujete pomoc s FileIT? Napište nám přes kontaktní formulář na file-app.uk.",
      },
      { property: "og:title", content: "Kontakt | FileIT" },
      {
        property: "og:description",
        content:
          "Máte dotaz, nápad nebo potřebujete pomoc s FileIT? Napište nám přes kontaktní formulář na file-app.uk.",
      },
      { property: "og:type", content: "website" },
      { property: "og:url", content: "https://file-app.uk/kontakt" },
      { name: "twitter:card", content: "summary_large_image" },
    ],
    links: [{ rel: "canonical", href: "https://file-app.uk/kontakt" }],
  }),
  component: ContactPage,
});

function ContactPage() {
  const [language, setLanguage] = useState<Language>("cs");
  const t = content[language];
  const submit = useServerFn(submitContactForm);
  const [status, setStatus] = useState<"idle" | "submitting" | "success" | "error">("idle");

  const schema = z.object({
    name: z.string().min(2, t.contact.form.errors.name).max(100),
    email: z.string().email(t.contact.form.errors.email).max(200),
    subject: z.string().max(200).optional(),
    message: z.string().min(10, t.contact.form.errors.message).max(5000),
  });

  type FormValues = z.infer<typeof schema>;

  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: "",
      email: "",
      subject: "",
      message: "",
    },
  });

  async function onSubmit(values: FormValues) {
    setStatus("submitting");
    try {
      const result = await submit({ data: values });
      if (result.success) {
        setStatus("success");
        form.reset();
      } else {
        setStatus("error");
      }
    } catch {
      setStatus("error");
    }
  }

  return (
    <div className="font-sans" lang={language === "cs" ? "cs" : "en"}>
      <header className="mesh-bg">
        <SiteHeader language={language} onChangeLanguage={setLanguage} t={t} />
        <div className="relative z-10 mx-auto max-w-6xl px-6 py-16 text-center sm:py-20">
          <h1 className="font-display text-4xl font-extrabold tracking-tight text-primary-foreground sm:text-5xl">
            {t.contact.title}
          </h1>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-primary-foreground/80">
            {t.contact.lead}
          </p>
        </div>
      </header>

      <main className="bg-background py-16 sm:py-20">
        <div className="mx-auto grid max-w-5xl gap-10 px-6 md:grid-cols-[1fr_1.25fr]">
          {/* Contact info */}
          <div className="space-y-8">
            <div>
              <div className="flex items-center gap-3">
                <span className="grid h-10 w-10 place-items-center rounded-xl bg-brand-pink/12 text-brand-pink-deep">
                  <MessageSquare className="h-5 w-5" />
                </span>
                <h2 className="font-display text-2xl font-bold text-foreground">
                  {t.contact.info.heading}
                </h2>
              </div>
              <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                {t.contact.info.response}
              </p>
            </div>

            <div className="space-y-4">
              <a
                href="mailto:info@file-app.uk"
                className="flex items-center gap-4 rounded-2xl border border-border bg-card p-4 shadow-[var(--shadow-card)] transition-colors hover:border-brand-pink/30"
              >
                <span className="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-brand-pink/12 text-brand-pink-deep">
                  <Mail className="h-5 w-5" />
                </span>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    {t.contact.info.email}
                  </p>
                  <p className="font-medium text-foreground">info@file-app.uk</p>
                </div>
              </a>

              <a
                href="https://github.com/jardaKarlik/fileIT"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-4 rounded-2xl border border-border bg-card p-4 shadow-[var(--shadow-card)] transition-colors hover:border-brand-pink/30"
              >
                <span className="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-brand-violet/10 text-brand-violet">
                  <Github className="h-5 w-5" />
                </span>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    {t.contact.info.github}
                  </p>
                  <p className="font-medium text-foreground">github.com/jardaKarlik/fileIT</p>
                </div>
              </a>

              <a
                href="https://fileit.featurebase.app/"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-4 rounded-2xl border border-border bg-card p-4 shadow-[var(--shadow-card)] transition-colors hover:border-brand-pink/30"
              >
                <span className="grid h-11 w-11 shrink-0 place-items-center rounded-xl bg-brand-coral/15 text-brand-coral">
                  <Heart className="h-5 w-5" />
                </span>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    {t.contact.info.featurebase}
                  </p>
                  <p className="font-medium text-foreground">fileit.featurebase.app</p>
                </div>
              </a>
            </div>
          </div>

          {/* Form */}
          <Card className="shadow-[var(--shadow-float)]">
            <CardContent className="p-6 sm:p-8">
              {status === "success" ? (
                <div className="flex flex-col items-center py-10 text-center">
                  <span className="grid h-16 w-16 place-items-center rounded-full bg-status-green/15 text-status-green">
                    <CheckCircle2 className="h-8 w-8" />
                  </span>
                  <h3 className="mt-5 font-display text-xl font-bold text-foreground">
                    {t.contact.form.success}
                  </h3>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {t.contact.form.successDetail}
                  </p>
                  <Button
                    onClick={() => setStatus("idle")}
                    className="mt-6 rounded-full bg-brand-pink font-bold text-primary-foreground hover:bg-brand-pink-deep"
                  >
                    {t.contact.form.submit}
                  </Button>
                </div>
              ) : (
                <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-5">
                  <div className="grid gap-5 sm:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="name">{t.contact.form.name}</Label>
                      <Input
                        id="name"
                        placeholder={t.contact.form.namePlaceholder}
                        {...form.register("name")}
                      />
                      {form.formState.errors.name && (
                        <p className="text-xs text-destructive">
                          {form.formState.errors.name.message}
                        </p>
                      )}
                    </div>

                    <div className="space-y-2">
                      <Label htmlFor="email">{t.contact.form.email}</Label>
                      <Input
                        id="email"
                        type="email"
                        placeholder={t.contact.form.emailPlaceholder}
                        {...form.register("email")}
                      />
                      {form.formState.errors.email && (
                        <p className="text-xs text-destructive">
                          {form.formState.errors.email.message}
                        </p>
                      )}
                    </div>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="subject">{t.contact.form.subject}</Label>
                    <Input
                      id="subject"
                      placeholder={t.contact.form.subjectPlaceholder}
                      {...form.register("subject")}
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="message">{t.contact.form.message}</Label>
                    <Textarea
                      id="message"
                      placeholder={t.contact.form.messagePlaceholder}
                      rows={5}
                      {...form.register("message")}
                    />
                    {form.formState.errors.message && (
                      <p className="text-xs text-destructive">
                        {form.formState.errors.message.message}
                      </p>
                    )}
                  </div>

                  {status === "error" && (
                    <p className="text-sm text-destructive">{t.contact.form.error}</p>
                  )}

                  <Button
                    type="submit"
                    disabled={status === "submitting"}
                    className="w-full rounded-full bg-brand-pink font-bold text-primary-foreground hover:bg-brand-pink-deep"
                  >
                    {status === "submitting" ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : null}
                    {status === "submitting" ? t.contact.form.sending : t.contact.form.submit}
                  </Button>
                </form>
              )}
            </CardContent>
          </Card>
        </div>
      </main>

      <SiteFooter t={t} />
    </div>
  );
}
