# RabbitMQ

It is assumed that we are going to use RabbitMQ queues, not streams, to simplify this first iteration. If the need gets identified, we may switch to streams.

BELOW ARE EXAMPLES TO MODIFY:

## Producers

(**Draft example** for the messages service) This service produces events related to the lifecycle of messages.

ProduceMessageCreated:

```txt
key: messages.created
exchange name and type: `messages.events` of type Topic
message: CreateMessage message in messages.proto in the events-protobuf repository, Protobuf package `messages.events`.
```

## Consumers

(**Draft example** for the notifications service) This service listens for incoming messages in order to send notifications to user.

ConsumeMessageCreated:

```txt
queue: notifications.messages.created
exchange name and type: `messages.events` of type Topic
binding: messages.created
```

Notice the naming convention for consumer queues is <consumer-service>.<domain-or-event-produced>.<action>

---

More information on exchange types: <https://www.rabbitmq.com/docs/exchanges>  
For example, an exchange of type "Topic" can use patterns in the binding, such as `binding: messages.*`
