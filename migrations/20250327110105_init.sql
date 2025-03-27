create extension if not exists "uuid-ossp";

create table if not exists securities
(
    id uuid primary key not null default uuid_generate_v4(),
    code varchar(255) not null
);

create table if not exists candles 
(
    id uuid primary key not null default uuid_generate_v4(),
    security_id uuid not null references public.securities(id) on delete cascade,
    open decimal not null default 0,
    close decimal not null default 0,
    high decimal not null default 0,
    low decimal not null default 0,
    value decimal not null default 0,
    volume decimal not null default 0,
    begin_t timestamp without time zone not null,
    end_t timestamp without time zone not null
);
