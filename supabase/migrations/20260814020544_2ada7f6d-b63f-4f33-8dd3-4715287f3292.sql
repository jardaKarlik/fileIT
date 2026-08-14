CREATE TABLE public.contact_submissions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name text NOT NULL,
  email text NOT NULL,
  subject text,
  message text NOT NULL,
  resolved boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now()
);

GRANT INSERT ON public.contact_submissions TO anon;
GRANT INSERT ON public.contact_submissions TO authenticated;
GRANT ALL ON public.contact_submissions TO service_role;

ALTER TABLE public.contact_submissions ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Allow anonymous submissions" ON public.contact_submissions
FOR INSERT TO public
WITH CHECK (true);