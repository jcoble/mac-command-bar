-- The callback stores the hash of a one-time code it hands only to the app
-- through its deep link; a claim must present that code.
ALTER TABLE oauth_attempts ADD COLUMN claim_code_hash TEXT;
