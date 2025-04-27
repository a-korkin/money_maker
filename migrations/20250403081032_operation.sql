create table if not exists attempts 
(
    id uuid primary key not null default uuid_generate_v4(),
    created_at timestamp without time zone not null default now(),
    profit decimal not null default 0
);

create table if not exists operations
(
    id uuid primary key not null default uuid_generate_v4(),
    attempt_id uuid not null references public.attempts(id) on delete cascade, 
    operation_type varchar(255) not null,
    security_id uuid not null references public.securities(id) on delete cascade,
    count integer not null default 0,
    price decimal not null default 0,
    commission decimal not null default 0,
    time_at timestamp without time zone not null default now(),
    sum_before decimal not null default 0,
    sum_after decimal not null default 0,
    prev uuid
);
