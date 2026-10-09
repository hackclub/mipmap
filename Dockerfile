FROM rust:1-bookworm AS build

COPY --from=node:22-bookworm-slim /usr/local/bin/node /usr/local/bin/node
COPY --from=node:22-bookworm-slim /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx

WORKDIR /app

COPY frontend/package.json frontend/package-lock.json frontend/.npmrc frontend/
RUN npm --prefix frontend ci

COPY . .

ARG VITE_SITE_URL
ARG VITE_DOCS_VIDEO_URL
ENV VITE_SITE_URL=$VITE_SITE_URL VITE_DOCS_VIDEO_URL=$VITE_DOCS_VIDEO_URL

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home mipmap

WORKDIR /app
COPY --from=build /app/target/release/mipmap /app/mipmap
COPY --from=build /app/frontend/build /app/frontend/build

USER mipmap
EXPOSE 8080
CMD ["/app/mipmap"]
