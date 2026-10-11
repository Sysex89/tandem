FROM nginx:alpine

RUN apk add --no-cache openssl && \
    mkdir -p /etc/nginx/ssl && \
    openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout /etc/nginx/ssl/nginx-selfsigned.key \
    -out /etc/nginx/ssl/nginx-selfsigned.crt \
    -subj "/C=BE/ST=Flanders/L=Antwerp/O=42/OU=transcendence/CN=localhost"

RUN rm /etc/nginx/conf.d/default.conf

COPY conf/nginx.conf /etc/nginx/conf.d/
COPY conf/api-locations.conf /etc/nginx/snippets/
# COPY conf/index.html /var/www/html/
COPY conf/custom_502.html /var/www/html/502.html

EXPOSE 443

CMD ["nginx", "-g", "daemon off;"]