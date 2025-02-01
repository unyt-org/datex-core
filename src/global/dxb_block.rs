use super::dxb_header::DXBHeader;

#[derive(Debug, Clone, Default)]
pub struct DXBBlock {
    pub header: DXBHeader,
    pub body: Vec<u8>,
}


impl DXBBlock {

    pub fn new(header: DXBHeader, body: Vec<u8>) -> DXBBlock {
        DXBBlock {
            header,
            body,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let pre_header = &mut self.header.pre_header_to_bytes();
		let block_header = self.header.block_header_to_bytes();

		pre_header.extend_from_slice(&block_header);        
        pre_header.extend_from_slice(&self.body);
        return pre_header.to_vec();
    }

}