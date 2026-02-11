-- Create the postgres role used by init.sql and test queries
-- Supabase image only has supabase_admin by default
create role postgres superuser login password 'postgres';
