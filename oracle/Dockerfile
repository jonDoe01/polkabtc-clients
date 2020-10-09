# ===== FIRST STAGE ======

FROM phusion/baseimage:0.11 as builder
ENV DEBIAN_FRONTEND=noninteractive
ARG PROFILE=release
WORKDIR /app

COPY . /app

RUN apt-get update && \
	apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
	apt-get install -y cmake pkg-config libssl-dev git clang

ARG TOOLCHAIN=nightly-2020-10-01

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install ${TOOLCHAIN} && \
	rustup target add wasm32-unknown-unknown --toolchain ${TOOLCHAIN} && \
	rustup default stable && \
	cargo build "--$PROFILE" --package oracle

# ===== SECOND STAGE ======

FROM phusion/baseimage:0.11
ARG PROFILE=release

RUN mv /usr/share/ca* /tmp && \
	rm -rf /usr/share/*  && \
	mv /tmp/ca-certificates /usr/share/

COPY --from=builder /app/target/$PROFILE/oracle /usr/local/bin

# Checks
RUN ldd /usr/local/bin/oracle && \
	/usr/local/bin/oracle --version

# Shrinking
RUN rm -rf /usr/lib/python* && \
	rm -rf /usr/bin /usr/sbin /usr/share/man

CMD ["/usr/local/bin/oracle"]