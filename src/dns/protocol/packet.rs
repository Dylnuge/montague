use super::{bigendians, DnsFlags, DnsFormatError, DnsQuestion, DnsResourceRecord};

#[derive(Clone, PartialEq, Debug)]
pub struct DnsPacket {
    // DNS transaction ID is a 16 bit number. It's arbitrary when transmitted
    // and copied into the reply so the client knows which replies correspond
    // to which requests if it's asking the same DNS server multiple questions.
    pub id: u16,
    // 16 more bits for flags which tell us a lot about the DNS packet.
    pub flags: DnsFlags,
    // u16 for number of: questions (QDCOUNT), answers (ANCOUNT), nameserver
    // records (NSCOUNT), and additional records (ARCOUNT), followed by each
    // of those segments in order
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsResourceRecord>,
    pub nameservers: Vec<DnsResourceRecord>,
    pub addl_recs: Vec<DnsResourceRecord>,
}

impl DnsPacket {
    pub fn from_bytes(bytes: &[u8]) -> Result<DnsPacket, DnsFormatError> {
        let id: u16;
        let flags: DnsFlags;
        let qd_count: u16;
        let an_count: u16;
        let ns_count: u16;
        let ar_count: u16;
        let mut questions: Vec<DnsQuestion> = Vec::new();
        let mut answers: Vec<DnsResourceRecord> = Vec::new();
        let mut nameservers: Vec<DnsResourceRecord> = Vec::new();
        let mut addl_recs: Vec<DnsResourceRecord> = Vec::new();

        if bytes.len() < 12 {
            return Err(DnsFormatError::make_error(format!(
                "Packet has incomplete header; only {} bytes received",
                bytes.len()
            )));
        }

        // TODO(dylan): Error checking, e.g. DNS request too short
        // Read the first two bytes as a big-endian u16 containing transaction id
        id = bigendians::to_u16(&bytes[0..2]);
        // Next two bytes are flags
        // If we get an error parsing the flags, we have too little info to
        // return a FormErr; we could just copy the bad flags but technically a
        // FormErr indicates an issue with the query, not the flags.
        flags = DnsFlags::from_bytes(&bytes[2..4])?;
        // Counts are next four u16s (big-endian)
        qd_count = bigendians::to_u16(&bytes[4..6]);
        an_count = bigendians::to_u16(&bytes[6..8]);
        ns_count = bigendians::to_u16(&bytes[8..10]);
        ar_count = bigendians::to_u16(&bytes[10..12]);

        // The header was 12 bytes, we now begin reading the rest of the packet.
        // These components are variable length (thanks to how labels are
        // encoded)
        let mut pos: usize = 12;
        for _ in 0..qd_count {
            // TODO(dylan): formerr logic is duplicated several times here,
            // might be helpful to turn it into a macro
            match DnsQuestion::from_bytes(&bytes, pos) {
                Ok((question, new_pos)) => {
                    pos = new_pos;
                    questions.push(question);
                }
                Err(mut form_err) => {
                    form_err.set_partial(DnsPacket {
                        id,
                        flags,
                        questions,
                        answers,
                        nameservers,
                        addl_recs,
                    });
                    return Err(form_err);
                }
            }
        }

        for _ in 0..an_count {
            match DnsResourceRecord::from_bytes(&bytes, pos) {
                Ok((rr, new_pos)) => {
                    pos = new_pos;
                    answers.push(rr);
                }
                Err(mut form_err) => {
                    form_err.set_partial(DnsPacket {
                        id,
                        flags,
                        questions,
                        answers,
                        nameservers,
                        addl_recs,
                    });
                    return Err(form_err);
                }
            }
        }

        for _ in 0..ns_count {
            match DnsResourceRecord::from_bytes(&bytes, pos) {
                Ok((rr, new_pos)) => {
                    pos = new_pos;
                    nameservers.push(rr);
                }
                Err(mut form_err) => {
                    form_err.set_partial(DnsPacket {
                        id,
                        flags,
                        questions,
                        answers,
                        nameservers,
                        addl_recs,
                    });
                    return Err(form_err);
                }
            }
        }

        for _ in 0..ar_count {
            match DnsResourceRecord::from_bytes(&bytes, pos) {
                Ok((rr, new_pos)) => {
                    pos = new_pos;
                    addl_recs.push(rr);
                }
                Err(mut form_err) => {
                    form_err.set_partial(DnsPacket {
                        id,
                        flags,
                        questions,
                        answers,
                        nameservers,
                        addl_recs,
                    });
                    return Err(form_err);
                }
            }
        }

        Ok(DnsPacket {
            id,
            flags,
            questions,
            answers,
            nameservers,
            addl_recs,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(&bigendians::from_u16(self.id));
        bytes.extend_from_slice(&self.flags.to_bytes());
        bytes.extend_from_slice(&bigendians::from_u16(self.questions.len() as u16));
        bytes.extend_from_slice(&bigendians::from_u16(self.answers.len() as u16));
        bytes.extend_from_slice(&bigendians::from_u16(self.nameservers.len() as u16));
        bytes.extend_from_slice(&bigendians::from_u16(self.addl_recs.len() as u16));

        for question in &self.questions {
            bytes.extend_from_slice(&question.to_bytes());
        }
        for answer in &self.answers {
            bytes.extend_from_slice(&answer.to_bytes());
        }
        for nameserver in &self.nameservers {
            bytes.extend_from_slice(&nameserver.to_bytes());
        }
        for addl_rec in &self.addl_recs {
            bytes.extend_from_slice(&addl_rec.to_bytes());
        }

        bytes
    }
}
