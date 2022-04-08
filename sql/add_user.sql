INSERT INTO default_schema.users
  (username, email, first_name, last_name)
VALUES 
  ($1, $2, $3, $4)
RETURNING 
  $table_fields;
