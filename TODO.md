# TODO

- [x] use TLS for signaling, DTLS-SRTP for media.

- [x] add shell script wrapper:
    - reset database
    - run database migration
    - mock database with 
    - user1 password user1user1 (owner) 
    - user2 (admin)
    - user3 (member)
    - text channel 1 and 2
    - voice channel 1 and 2
    - mock dm messages between user1 -> user2, user1 -> user3

- [x] show mic and deafen status of users in Voice Channels. use emoji for user voice status.

- [x] responsive ui in mobile.

- [x] bubble chat message. other on left, us on right.

- [x] auto focus message input.

- [x] move direct message to new page. add menu to navbar. show unread message count badge.

- [x] add sound for voice connect and disconnect.

- [x] add sound for new message.

- [x] fix error "Failed to negotiate voice renegotiation.". show debug log on devtools console. 
    reproduce:
    - user1 join VC.
    - user2 join VC.
    - user2 disconnect VC. 
    - user2 join VC again.
    - user1 have error "Failed to negotiate voice renegotiation.".
