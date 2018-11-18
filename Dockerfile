FROM debian:stable-slim

ARG env=debug

RUN mkdir -p /app/config \
  && apt-get update \
  && apt-get install -y wget gnupg2 ca-certificates \
  && sh -c 'wget -q https://www.postgresql.org/media/keys/ACCC4CF8.asc -O - | apt-key add -' \
  && sh -c 'echo "deb http://apt.postgresql.org/pub/repos/apt/ stretch-pgdg main" >> /etc/apt/sources.list.d/pgdg.list' \
  && apt-get update \
  && apt-get install -y libpq5 libmariadbclient18 \
  && apt-get purge -y wget \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/ \
  && adduser --disabled-password --gecos "" --home /app --no-create-home -u 5000 app \
  && chown -R app: /app

COPY target/$env/payments_notifications /app
COPY config /app/config

USER app
WORKDIR /app

EXPOSE 8000

ENTRYPOINT ["/app/payments_notifications", "server"]
