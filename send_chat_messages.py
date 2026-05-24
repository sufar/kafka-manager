import json
import random
import time
import uuid
from datetime import datetime, timedelta

from kafka import KafkaProducer

BOOTSTRAP_SERVERS = ["localhost:9092"]
TOPIC = "chat-messages"
TOTAL = 10000

USERS = [
    "alice", "bob", "charlie", "diana", "eve",
    "frank", "grace", "hank", "iris", "jack",
    "kate", "leo", "mia", "nina", "oscar",
]

CHANNELS = [
    "general", "random", "engineering", "design",
    "marketing", "support", "announcements", "dev-help",
]

GREETINGS = [
    "Hey everyone!", "Hi there!", "Good morning!", "Hello!",
    "What's up?", "Morning team!", "Happy Friday!", "Hi all,",
]

MESSAGES = [
    "Has anyone tried the new feature yet?",
    "I think we should refactor the auth module.",
    "Can someone review my PR #452?",
    "The deployment went smoothly!",
    "Anyone up for lunch?",
    "Just pushed the latest changes to main.",
    "We have a meeting at 3pm today.",
    "Found a bug in the search component.",
    "Great work on the sprint!",
    "Let's sync up after standup.",
    "The CI pipeline is failing again.",
    "Who owns the payment service?",
    "I'll take a look at the logs.",
    "The new dashboard looks amazing!",
    "Can we add pagination to the API?",
    "Need help with the database migration.",
    "Updated the README with new instructions.",
    "The load test results are promising.",
    "Should we upgrade to the latest version?",
    "Just finished the onboarding flow.",
    "Remember to update your API keys.",
    "The test coverage dropped to 78%.",
    "Let's discuss in the next retro.",
    "I'm seeing 500 errors in staging.",
    "Nice catch on that edge case!",
    "The Docker image size is too large.",
    "Can someone help with the SSL cert?",
    "We hit 10k daily active users!",
    "The cache invalidation is tricky here.",
    "I'll write up a design doc for this.",
]

REPLIES = [
    "Agreed!", "Good point.", "I'll look into it.", "Sounds good to me.",
    "Let me check.", "On it!", "Thanks for sharing.", "Makes sense.",
    "I noticed that too.", "Will do.", "Already working on it.",
    "Let's discuss offline.", "👍", "Great idea!", "I have some concerns.",
]


def generate_message(idx):
    base_time = datetime.now() - timedelta(hours=TOTAL - idx)
    user = random.choice(USERS)
    channel = random.choice(CHANNELS)

    msg_type = random.choices(
        ["chat", "reply", "system"],
        weights=[60, 30, 10],
    )[0]

    if msg_type == "chat":
        content = random.choice(GREETINGS + MESSAGES)
    elif msg_type == "reply":
        content = random.choice(REPLIES)
    else:
        content = random.choice([
            "User joined the channel.",
            "User left the channel.",
            "Channel settings updated.",
        ])

    payload = {
        "message_id": str(uuid.uuid4()),
        "timestamp": base_time.isoformat() + "Z",
        "user": user,
        "channel": channel,
        "type": msg_type,
        "content": content,
        "metadata": {
            "client": random.choice(["web", "mobile-ios", "mobile-android", "desktop"]),
            "ip": f"192.168.{random.randint(1,254)}.{random.randint(1,254)}",
            "sequence": idx,
        },
    }

    if msg_type == "reply":
        payload["reply_to"] = str(uuid.uuid4())

    return json.dumps(payload)


def main():
    producer = KafkaProducer(
        bootstrap_servers=BOOTSTRAP_SERVERS,
        value_serializer=lambda v: v.encode("utf-8"),
        acks="all",
        retries=3,
    )

    print(f"Sending {TOTAL} messages to {TOPIC} @ {BOOTSTRAP_SERVERS} ...")
    t0 = time.time()

    for i in range(1, TOTAL + 1):
        msg = generate_message(i)
        producer.send(TOPIC, value=msg)

        if i % 1000 == 0:
            producer.flush()
            elapsed = time.time() - t0
            print(f"  Sent {i}/{TOTAL} ({i/elapsed:.0f} msg/s)")

    producer.flush()
    producer.close()

    total_time = time.time() - t0
    print(f"\nDone! {TOTAL} messages sent in {total_time:.2f}s ({TOTAL/total_time:.0f} msg/s)")


if __name__ == "__main__":
    main()
