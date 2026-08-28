FROM python:3.11-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*
COPY platform/mox-server/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY platform/mox-server/ ./
COPY tools/ /app/tools/
RUN mkdir -p /app/data
EXPOSE 8600
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl -f http://localhost:8600/api/health || exit 1
CMD ["python", "run.py", "8600"]
