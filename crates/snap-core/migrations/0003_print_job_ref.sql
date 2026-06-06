-- Print job ids from CUPS / mock are strings (e.g. "PRINTER-42"), so add a
-- text reference column alongside the original integer one.
ALTER TABLE print_jobs ADD COLUMN job_ref TEXT;
