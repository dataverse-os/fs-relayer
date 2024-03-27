# Dataverse File Relayer

## API Documentation

Endpoints: `https://file-relayer.dataverse.art`

### GET /dataverse/stream

Load a specific file stream.

#### Parameters

- `stream_id`: StreamId (required)
- `dapp_id`: uuid::Uuid (required)
- `format`: String (optional), cloud be null or `ceramic`

#### Responses

- `200 OK`: Returns the requested stream.
- `400 Bad Request`: If there is an error loading the stream.

---

### GET /dataverse/streams

Load multiple file streams.

#### Parameters

- `model_id`: StreamId (required)
- `account`: String (optional)

#### Responses

- `200 OK`: Returns the requested streams.
- `400 Bad Request`: If there is an error loading the streams.

---

### POST /dataverse/stream

Create a new stream.

#### Parameters

- `dapp_id`: uuid::Uuid (required)
- Payload: commit::Genesis (required)

#### Responses

- `200 OK`: Returns the created stream.
- `400 Bad Request`: If there is an error creating the stream.

---

### PUT /dataverse/stream

Update an existing stream.

#### Parameters

- `dapp_id`: uuid::Uuid (required)
- Payload: commit::Data (required)

#### Responses

- `200 OK`: Returns the updated stream.
- `400 Bad Request`: If there is an error updating the stream.
