# =============================================================================
# ⚡ PHP-Hyperion Production Container Deployment
# Ultra-High Performance PHP 8.4 Runtime Engine
# =============================================================================

FROM php:8.4-cli-alpine AS runtime

# Install essential runtime libraries
RUN apk add --no-cache \
    bash \
    curl \
    git \
    libstdc++ \
    sqlite-libs \
    sqlite-dev \
    libpng-dev \
    libjpeg-turbo-dev \
    freetype-dev \
    libzip-dev \
    icu-dev \
    oniguruma-dev \
    && docker-php-ext-install -j$(nproc) \
    pdo \
    pdo_sqlite \
    pdo_mysql \
    opcache \
    mbstring \
    intl \
    zip \
    bcmath

# Copy deployed Hyperion binary & worker engine
COPY deploy/hyperion /usr/local/bin/hyperion
COPY hyperion_worker.php /usr/local/share/hyperion/hyperion_worker.php
COPY deploy/hyperion.service /etc/hyperion/hyperion.service
COPY deploy/sysctl.conf /etc/sysctl.d/99-hyperion.conf

RUN chmod +x /usr/local/bin/hyperion

WORKDIR /app

EXPOSE 8000 9000

ENV APP_ENV=production \
    HYPERION_REACTORS=8 \
    HYPERION_MAX_FIBRES=32

ENTRYPOINT ["hyperion"]
CMD ["-S", "0.0.0.0:8000", "-W"]
