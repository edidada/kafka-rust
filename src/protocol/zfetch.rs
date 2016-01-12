//! An efficient response parser for the "fetch messages" use-case.

use std::borrow::Cow;
use std::io::Read;
use std::mem;
use std::slice::Iter;

use error::{Error, Result};
use compression::{gzip, Compression};
use compression::snappy::SnappyReader;

use super::FromResponse;
use super::zreader::ZReader;

// ~ helper macro to aid parsing arrays of values (as defined by the
// Kafka protocol.)
macro_rules! array_of {
    ($zreader:ident, $parse_elem:expr) => {{
        let n_elems = try!($zreader.read_array_len());
        let mut array = Vec::with_capacity(n_elems);
        for _ in 0..n_elems {
            array.push(try!($parse_elem));
        }
        array
    }}
}

/// The result of a "fetch messages" request from a particular Kafka
/// broker. Such a response can contain messages for multiple topic
/// partitions.
pub struct FetchResponse {
    // used to "own" the data all other references of this struct
    // point to.
    #[allow(dead_code)]
    raw_data: Vec<u8>,

    correlation_id: i32,

    // ~ static is used here to get around the fact that we don't want
    // FetchResponse have a lifetime parameter as well. the field is
    // exposed only through an accessor which binds the exposed
    // lifetime to the lifetime of the FetchResponse instance
    topics: Vec<TopicFetchResponse<'static>>,
}

impl FromResponse for FetchResponse {
    fn from_response(response: Vec<u8>) -> Result<Self> {
        FetchResponse::from_vec(response)
    }
}

impl FetchResponse {
    /// Parses a FetchResponse from binary data as defined by the
    /// Kafka Protocol.
    fn from_vec(response: Vec<u8>) -> Result<FetchResponse> {
        let slice = unsafe { mem::transmute(&response[..]) };
        let mut r = ZReader::new(slice);
        let correlation_id = try!(r.read_i32());
        let topics = array_of!(r, TopicFetchResponse::read(&mut r));
        Ok(FetchResponse {
            raw_data: response,
            correlation_id: correlation_id,
            topics: topics
        })
    }

    /// Retrieves the id corresponding to the fetch messages request
    /// (provided for debugging purposes only).
    #[inline]
    pub fn correlation_id(&self) -> i32 {
        self.correlation_id
    }

    /// Provides an iterator over all the topics and the fetched data
    /// relative to these topics.
    #[inline]
    pub fn topics<'a>(&'a self) -> &[TopicFetchResponse<'a>] {
        &self.topics
    }
}

/// The result of a "fetch messages" request from a particular Kafka
/// broker for a single topic only.  Beside the name of the topic,
/// this structure provides an iterator over the topic partitions from
/// which messages were requested.
pub struct TopicFetchResponse<'a> {
    topic: &'a str,
    partitions: Vec<PartitionFetchResponse<'a>>,
}

impl<'a> TopicFetchResponse<'a> {
    fn read(r: &mut ZReader<'a>) -> Result<TopicFetchResponse<'a>> {
        let name = try!(r.read_str());
        let partitions = array_of!(r, PartitionFetchResponse::read(r));
        Ok(TopicFetchResponse {
            topic: name,
            partitions: partitions,
        })
    }

    /// Retrieves the identifier/name of the represented topic.
    #[inline]
    pub fn topic(&self) -> &'a str {
        self.topic
    }

    /// Provides an iterator over all the partitions of this topic for
    /// which messages were requested.
    #[inline]
    pub fn partitions(&self) -> &[PartitionFetchResponse<'a>] {
        &self.partitions
    }
}

/// The result of a "fetch messages" request from a particular Kafka
/// broker for a single topic partition only.  Beside the partition
/// identifier, this structure provides an iterator over the actually
/// requested message data.
///
/// Note: There might have been a (recoverable) error for a particular
/// partition (but not for another).
pub struct PartitionFetchResponse<'a> {
    /// The identifier of the represented partition.
    partition: i32,

    /// Either an error or the partition data.
    data: Result<PartitionData<'a>>,
}

impl<'a> PartitionFetchResponse<'a> {
    fn read(r: &mut ZReader<'a>) -> Result<PartitionFetchResponse<'a>> {
        let partition = try!(r.read_i32());
        let error = Error::from_protocol_error(try!(r.read_i16()));
        // we need to parse the rest even if there was an error to
        // consume the input stream (zreader)
        let highwatermark = try!(r.read_i64());
        let msgset = try!(MessageSet::from_slice(try!(r.read_bytes())));
        Ok(PartitionFetchResponse {
            partition: partition,
            data: match error {
                Some(error) => Err(error),
                None => Ok(PartitionData {
                    highwatermark_offset: highwatermark,
                    message_set: msgset,
                }),
            },
        })
    }

    /// Retrieves the identifier of the represented partition.
    #[inline]
    pub fn partition(&self) -> i32 {
        self.partition
    }

    /// Retrieves the data payload for this partition.
    pub fn data(&'a self) -> &'a Result<PartitionData<'a>> {
        &self.data
    }
}

/// The successfully fetched data payload for a particular partition.
pub struct PartitionData<'a> {
    highwatermark_offset: i64,
    message_set: MessageSet<'a>,
}

impl<'a> PartitionData<'a> {
    /// Retrieves the so-called "high water mark offset" indicating
    /// the "latest" offset for this partition at the remote broker.
    /// This can be used by clients to find out how much behind the
    /// latest available message they are.
    #[inline]
    pub fn highwatermark_offset(&self) -> i64 {
        self.highwatermark_offset
    }

    /// Retrieves the fetched message data for this partition.
    #[inline]
    pub fn messages(&self) -> &[Message<'a>] {
        return &self.message_set.messages
    }
}

struct MessageSet<'a> {
    #[allow(dead_code)]
    raw_data: Cow<'a, [u8]>, // ~ this field is used to potentially "own" the underlying vector
    messages: Vec<Message<'a>>,
}

/// A fetched messages from a remote Kafka broker for a particular
/// topic partition.
pub struct Message<'a> {
    /// The offset at which this message resides in the remote kafka
    /// broker topic partition.
    pub offset: i64,

    /// The "key" data of this message.  Empty if there is no such
    /// data for this message.
    pub key: &'a [u8],

    /// The value data of this message.  Empty if there is no such
    /// data for this message.
    pub value: &'a [u8],
}

impl<'a> MessageSet<'a> {
    fn from_vec(data: Vec<u8>) -> Result<MessageSet<'a>> {
        // since we're going to keep the original
        // uncompressed vector around without
        // further modifying it and providing
        // publicly no mutability possibilities
        // this is safe
        let ms = try!(MessageSet::from_slice(unsafe {
            mem::transmute(&data[..])
        }));
        return Ok(MessageSet {
            raw_data: Cow::Owned(data),
            messages: ms.messages,
        });
    }

    fn from_slice<'b>(raw_data: &'b [u8]) -> Result<MessageSet<'b>> {
        let mut r = ZReader::new(raw_data);
        let mut msgs = Vec::new();
        while !r.is_empty() {
            match MessageSet::next_message(&mut r) {
                // this is the last messages which might be
                // incomplete; a valid case to be handled by
                // consumers
                Err(Error::UnexpectedEOF) => {
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
                Ok((offset, pmsg)) => {
                    // handle compression (denoted by the last 2 bits
                    // of the attr field)
                    match pmsg.attr & 0x03 {
                        c if c == Compression::NONE as i8 => {
                            msgs.push(Message {
                                offset: offset,
                                key: pmsg.key,
                                value: pmsg.value
                            });
                        }
                        // XXX handle recursive compression in future
                        c if c == Compression::GZIP as i8 => {
                            let v = try!(gzip::uncompress(pmsg.value));
                            return Ok(try!(MessageSet::from_vec(v)));
                        }
                        c if c == Compression::SNAPPY as i8 => {
                            let mut v = Vec::new();
                            try!(try!(SnappyReader::new(pmsg.value)).read_to_end(&mut v));
                            return Ok(try!(MessageSet::from_vec(v)));
                        }
                        _ => panic!("Unknown compression type!"),
                    }
                }
            };
        }
        Ok(MessageSet {
            raw_data: Cow::Borrowed(raw_data),
            messages: msgs,
        })
    }

    fn next_message<'b>(r: &mut ZReader<'b>) -> Result<(i64, ProtocolMessage<'b>)> {
        let offset = try!(r.read_i64());
        let msg_data = try!(r.read_bytes());
        Ok((offset, try!(ProtocolMessage::from_slice(msg_data))))
    }
}

/// Represents a messages exactly as defined in the protocol.
struct ProtocolMessage<'a> {
    crc: i32,
    magic: i8,
    attr: i8,
    key: &'a [u8],
    value: &'a [u8],
}

impl<'a> ProtocolMessage<'a> {
    /// Parses a raw message from the given byte slice.  Does _not_
    /// handle any compression.
    fn from_slice<'b>(raw_data: &'b [u8]) -> Result<ProtocolMessage<'b>> {
        let mut r = ZReader::new(raw_data);

        let msg_crc = try!(r.read_i32());
        // XXX later validate the checksum
        let msg_magic = try!(r.read_i8());
        // XXX validate that `msg_magic == 0`
        let msg_attr = try!(r.read_i8());
        let msg_key = try!(r.read_bytes());
        let msg_val = try!(r.read_bytes());

        debug_assert!(r.is_empty());

        Ok(ProtocolMessage {
            crc: msg_crc,
            magic: msg_magic,
            attr: msg_attr,
            key: msg_key,
            value: msg_val,
        })
    }
}

// --------------------------------------------------------------------

/// A convenience helper for iterating "fetch messages" responses.
pub struct ResponseIter<'a> {
    responses: Iter<'a, FetchResponse>,
    topics: Option<Iter<'a, TopicFetchResponse<'a>>>,
    curr_topic: &'a str,
    partitions: Option<Iter<'a, PartitionFetchResponse<'a>>>,
}

/// A responce for a set of messages from a single topic partition.
pub struct Response<'a> {
    /// The name of the topic this response corresponds to.
    pub topic: &'a str,
    /// The partition this response correponds to.
    pub partition: i32,
    /// Either an error or the set of messages retrieved for the
    /// underlying topic partition.
    pub data: &'a Result<PartitionData<'a>>,
}

/// Provide a partition level iterator over all specified fetch
/// responses. Since there might be failures on the level of
/// partitions, it is the lowest unit at which a response can be
/// processed. The returned iterator will iterate all partitions in
/// the given responses in their specified order.
///
/// # Examples
///
/// ```no_run
/// use kafka::client::KafkaClient;
/// use kafka::utils::TopicPartitionOffset;
/// use kafka::client::zfetch::iter_responses;
///
/// // Fetch some data for two topic partitions
/// let mut client = KafkaClient::new(vec!("localhost:9092".to_owned()));
/// client.load_metadata_all().unwrap();
/// let reqs = &[TopicPartitionOffset{ topic: "my-topic", partition: 0, offset: 0 },
///              TopicPartitionOffset{ topic: "my-topic-2", partition: 0, offset: 0 }];
/// let resps = client.zfetch_messages_multi(reqs).unwrap();
///
/// // Iterate all the responses for all the partitions specified to be fetched data from
/// for r in iter_responses(&resps) {
///   match r.data {
///     &Err(ref e) => println!("error for {}:{}: {}", r.topic, r.partition, e),
///     &Ok(ref data) => {
///       for msg in data.messages() {
///         println!("{}:{}: {:?}", r.topic, r.partition, msg.value);
///       }
///     }
///   }
/// }
/// ```
pub fn iter_responses<'a>(responses: &'a [FetchResponse]) -> ResponseIter<'a> {
    let mut responses = responses.iter();
    let mut topics = responses.next().map(|r| r.topics().iter());
    let (curr_topic, partitions) =
        topics.as_mut()
        .and_then(|t| t.next())
        .map_or((None, None), |t| (Some(t.topic()), Some(t.partitions().iter())));
    ResponseIter {
        responses: responses,
        topics: topics,
        curr_topic: curr_topic.unwrap_or(""),
        partitions: partitions,
    }
}

impl<'a> Iterator for ResponseIter<'a> {
    type Item = Response<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // ~ then the next available partition
        if let Some(p) = self.partitions.as_mut().and_then(|p| p.next()) {
            return Some(Response {
                topic: self.curr_topic,
                partition: p.partition(),
                data: p.data(),
            });
        }
        // ~ then the next available topic
        if let Some(t) = self.topics.as_mut().and_then(|t| t.next()) {
            self.curr_topic = t.topic();
            self.partitions = Some(t.partitions().iter());
            return self.next();
        }
        // ~ then the next available response
        if let Some(r) = self.responses.next() {
            self.curr_topic = "";
            self.topics = Some(r.topics().iter());
            return self.next();
        }
        // ~ finally we know there's nothing available anymore
        None
    }
}

// tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::str;

    use super::{FetchResponse, Message};

    static FETCH1_TXT: &'static str =
        include_str!("../../test-data/fetch1.txt");
    static FETCH1_FETCH_RESPONSE_NOCOMPRESSION_K0821: &'static [u8] =
        include_bytes!("../../test-data/fetch1.mytopic.1p.nocompression.kafka.0821");
    static FETCH1_FETCH_RESPONSE_SNAPPY_K0821: &'static [u8] =
        include_bytes!("../../test-data/fetch1.mytopic.1p.snappy.kafka.0821");
    static FETCH1_FETCH_RESPONSE_SNAPPY_K0822: &'static [u8] =
        include_bytes!("../../test-data/fetch1.mytopic.1p.snappy.kafka.0822");
    static FETCH1_FETCH_RESPONSE_GZIP_K0821: &'static [u8] =
        include_bytes!("../../test-data/fetch1.mytopic.1p.gzip.kafka.0821");

    fn into_messages<'a>(r: &'a FetchResponse) -> Vec<&'a Message<'a>> {
        let mut all_msgs = Vec::new();
        for t in r.topics() {
            for p in t.partitions() {
                match p.data() {
                    &Err(_) => {
                        println!("Skipping error partition: {}:{}", t.topic, p.partition);
                    }
                    &Ok(ref data) => {
                        all_msgs.extend(data.messages());
                    }
                }
            }
        }
        all_msgs
    }

    fn test_decode_new_fetch_response(msg_per_line: &str, response: Vec<u8>) {
        let resp = FetchResponse::from_vec(response);
        let resp = resp.unwrap();

        let original: Vec<_> = msg_per_line.lines().collect();

        // ~ response for exactly one topic expected
        assert_eq!(1, resp.topics.len());
        // ~ topic name
        assert_eq!("my-topic", resp.topics[0].topic);
        // ~ exactly one partition
        assert_eq!(1, resp.topics[0].partitions.len());
        // ~ the first partition
        assert_eq!(0, resp.topics[0].partitions[0].partition);
        // ~ no error
        assert!(resp.topics[0].partitions[0].data.is_ok());

        let msgs = into_messages(&resp);
        assert_eq!(original.len(), msgs.len());
        for (msg, orig) in msgs.into_iter().zip(original.iter()) {
            assert_eq!(str::from_utf8(msg.value).unwrap(), *orig);
        }
    }

    #[test]
    fn test_from_slice_nocompression_k0821() {
        test_decode_new_fetch_response(FETCH1_TXT, FETCH1_FETCH_RESPONSE_NOCOMPRESSION_K0821.to_owned());
    }

    #[test]
    fn test_from_slice_snappy_k0821() {
        test_decode_new_fetch_response(FETCH1_TXT, FETCH1_FETCH_RESPONSE_SNAPPY_K0821.to_owned());
    }

    #[test]
    fn test_from_slice_snappy_k0822() {
        test_decode_new_fetch_response(FETCH1_TXT, FETCH1_FETCH_RESPONSE_SNAPPY_K0822.to_owned());
    }

    #[test]
    fn test_from_slice_gzip_k0821() {
        test_decode_new_fetch_response(FETCH1_TXT, FETCH1_FETCH_RESPONSE_GZIP_K0821.to_owned());
    }

    #[cfg(feature = "nightly")]
    mod benches {
        use test::{black_box, Bencher};

        use super::super::FetchResponse;
        use super::into_messages;

        fn bench_decode_new_fetch_response(b: &mut Bencher, data: Vec<u8>) {
            b.bytes = data.len() as u64;
            b.iter(|| {
                let data = data.clone();
                let r = black_box(FetchResponse::from_vec(data).unwrap());
                let v = black_box(into_messages(&r));
                v.len()
            });
        }

        #[bench]
        fn bench_decode_new_fetch_response_nocompression_k0821(b: &mut Bencher) {
            bench_decode_new_fetch_response(b, super::FETCH1_FETCH_RESPONSE_NOCOMPRESSION_K0821.to_owned())
        }

        #[bench]
        fn bench_decode_new_fetch_response_snappy_k0821(b: &mut Bencher) {
            bench_decode_new_fetch_response(b, super::FETCH1_FETCH_RESPONSE_SNAPPY_K0821.to_owned())
        }

        #[bench]
        fn bench_decode_new_fetch_response_snappy_k0822(b: &mut Bencher) {
            bench_decode_new_fetch_response(b, super::FETCH1_FETCH_RESPONSE_SNAPPY_K0822.to_owned())
        }

        #[bench]
        fn bench_decode_new_fetch_response_gzip_k0821(b: &mut Bencher) {
            bench_decode_new_fetch_response(b, super::FETCH1_FETCH_RESPONSE_GZIP_K0821.to_owned())
        }
    }
}
