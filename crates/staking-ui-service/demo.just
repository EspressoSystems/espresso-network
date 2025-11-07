set working-directory := 'demo'

up *services:
    docker compose up -d {{services}}

down:
    docker compose down -v --remove-orphans

logs *services:
    docker compose logs {{services}}

pull:
    docker compose pull
