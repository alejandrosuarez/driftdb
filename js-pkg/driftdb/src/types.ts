export type Key = string
export type SequenceNumber = number

export type Action = {type: 'append' | 'replace' | 'relay'} | {type: 'compact', seq: SequenceNumber}

export interface SequenceValue {
    value: any
    seq: SequenceNumber
}

export type MessageFromDb = {
    type: 'push',
    key: Key,
    value: SequenceValue,
} | {
    type: 'init',
    data: Array<SequenceValue>,
    key: Key,
} | {
    type: 'error',
    message: string
} | {
    type: 'stream_size',
    key: Key,
    size: number
}

export type MessageToDb = {
    type: 'push'
    action: Action
    value: any
    key: Key
} | {
    type: 'get'
    key: Key
    seq: SequenceNumber
}

export type ConnectionStatus = {
    connected: false
} | {
    connected: true
    debugUrl: string
}