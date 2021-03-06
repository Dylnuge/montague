use super::{bigendians, names, DnsClass, DnsFormatError, DnsRRType};

#[derive(Clone, PartialEq, Debug)]
pub struct DnsQuestion {
    // A QName is split up as a series of labels. For instance, the FQDN
    // "blog.example.com." contains three labels, "blog", "example", and "com".
    // We could store this in a number of different ways internally; for now I'm
    // going with a vector of strings which represents the labels in order.
    // e.g. "blog.example.com." would be `vec!["blog", "example", "com"]`.
    pub qname: Vec<String>,
    // The type of records desired. In general, this is an RRType; there are
    // some RRTypes (like ANY) which are only valid in queries and not actual
    // resource records.
    pub qtype: DnsRRType,
    // The class of records desired, which is nearly always IN for internet.
    // Feels like a waste of a 16 bit int; probably this was intended for some
    // grander purpose long ago.
    pub qclass: DnsClass,
}

impl DnsQuestion {
    pub fn from_bytes(
        packet_bytes: &[u8],
        mut pos: usize,
    ) -> Result<(DnsQuestion, usize), DnsFormatError> {
        let (qname, new_pos) = names::deserialize_name(&packet_bytes, pos)?;
        if new_pos + 4 > packet_bytes.len() {
            return Err(DnsFormatError::make_error(format!(
                "End of packet parsing question"
            )));
        }
        let qtype_num = bigendians::to_u16(&packet_bytes[new_pos..new_pos + 2]);
        let qclass_num = bigendians::to_u16(&packet_bytes[new_pos + 2..new_pos + 4]);
        pos = new_pos + 4;

        let qtype = match num::FromPrimitive::from_u16(qtype_num) {
            Some(x) => Ok(x),
            None => Err(DnsFormatError::make_error(format!(
                "Invalid qtype value: {:x}",
                qtype_num
            ))),
        }?;

        let qclass = match DnsClass::from_u16(qclass_num) {
            Some(x) => Ok(x),
            None => Err(DnsFormatError::make_error(format!(
                "Invalid class value: {:x}",
                qclass_num
            ))),
        }?;

        let question = DnsQuestion {
            qname,
            qtype,
            qclass,
        };

        Ok((question, pos))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.append(&mut names::serialize_name(&self.qname));
        bytes.extend_from_slice(&bigendians::from_u16(self.qtype.to_owned() as u16));
        bytes.extend_from_slice(&bigendians::from_u16(self.qclass.to_u16()));

        bytes
    }
}
