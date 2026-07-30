set working-directory := 'demo'

up *services:
    docker compose up -d --wait {{services}}

down:
    docker compose down -v --remove-orphans

logs *services:
    docker compose logs {{services}}

pull:
    docker compose pull
