# Quantum Algorithm Tutor (S18): runs the simulator, tests, pipeline and notebook on any PC with Docker.
# The API key is NEVER baked into the image; pass it at run time (see README "Run with Docker").
FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=utf-8 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

WORKDIR /app

COPY requirements.txt requirements-docker.txt ./
RUN pip install -r requirements-docker.txt

COPY . .
RUN sed -i 's/\r$//' docker/entrypoint.sh \
 && chmod +x docker/entrypoint.sh \
 && useradd --create-home --uid 1000 tutor \
 && chown -R tutor:tutor /app
USER tutor

EXPOSE 8888
ENTRYPOINT ["/app/docker/entrypoint.sh"]
CMD ["selfcheck"]
