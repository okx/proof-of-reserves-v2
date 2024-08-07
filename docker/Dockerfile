# Use the official Ubuntu 22.04 as the base image
FROM ubuntu:22.04

# Set environment variables to avoid user interaction during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Update the package repository and install some basic packages
RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y \
    build-essential \
    curl \
    wget \
    git \
    vim \
    nano \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /workspace/por_v2

# install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# Set the environment variables for Rust
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy the content of the current directory to /app
COPY . /workspace/por_v2

# Set the default command to run when the container starts
CMD ["bash"]