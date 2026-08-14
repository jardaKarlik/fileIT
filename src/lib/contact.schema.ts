import { z } from "zod";

export const contactFormSchema = z.object({
  name: z.string().min(2).max(100),
  email: z.string().email().max(200),
  subject: z.string().max(200).optional(),
  message: z.string().min(10).max(5000),
});

export type ContactFormValues = z.infer<typeof contactFormSchema>;
