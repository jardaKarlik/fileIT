import { createServerFn } from "@tanstack/react-start";

import { supabase } from "@/integrations/supabase/client";

import { contactFormSchema } from "./contact.schema";

export type ContactFormResult =
  | { success: true }
  | { success: false; error: string };

export const submitContactForm = createServerFn({ method: "POST" })
  .validator((data) => contactFormSchema.parse(data))
  .handler(async ({ data }): Promise<ContactFormResult> => {
    const { error } = await supabase.from("contact_submissions").insert({
      name: data.name,
      email: data.email,
      subject: data.subject ?? null,
      message: data.message,
    });

    if (error) {
      console.error("Contact submission failed", error);
      return { success: false, error: "Database error" };
    }

    return { success: true };
  });
