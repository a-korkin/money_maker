docker_up:
	docker compose up -d
docker_down:
	docker compose down -v
migrate:
	sqlx migrate run
