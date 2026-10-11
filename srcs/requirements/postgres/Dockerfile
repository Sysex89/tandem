# official postgres image, alpine = smaller size
FROM postgres:18-alpine

# NOTE: EXPOSE 5432 is already written in the postgres container

# copy sql scripts to postgres magic folder
# runs automatically on first container start (when data volume is empty)
COPY init/ /docker-entrypoint-initdb.d/