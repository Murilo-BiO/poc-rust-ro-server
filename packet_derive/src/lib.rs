use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemStruct, Type, PathArguments, GenericArgument, Meta, LitStr, PathSegment};


fn get_packet_id(attrs: &[syn::Attribute]) -> syn::Path {
	let mut packet_id = syn::Path {
			leading_colon: None,
			segments: Default::default(),
	};

	// let mut packet_id = 0_u16;

	for attr in attrs {
		if !attr.path().is_ident("packet") {
			continue;
		}

		if let Meta::List(meta) = &attr.meta {
			if meta.tokens.is_empty() {
				continue;
			}
		}

		let _ = attr.parse_nested_meta(|meta| {
			if !meta.path.is_ident("id") {
				return Ok(());
			}
			
			let v = meta.value()?;
			// let v: LitInt = v.parse()?;

			// let v = v.base10_parse::<u16>()?;

			// packet_id = v;
			let v: LitStr = v.parse().unwrap();
			
			let segment = PathSegment {
				ident: syn::Ident::new("PacketId", meta.path.get_ident().unwrap().span()),
				arguments: Default::default(),
			};

			let ident = syn::Ident::new(&v.value(), v.span());
			println!("Ident: {}", ident);
			packet_id.segments.push(segment);
			packet_id.segments.push(PathSegment {
				ident,
				arguments: Default::default(),
			});

			Ok(())
		});

	}

	packet_id
}

#[proc_macro_derive(Packet, attributes(packet))]
pub fn packet_derive(input: TokenStream) -> TokenStream {
    let input: ItemStruct = parse_macro_input!(input as ItemStruct);

    let struct_name = &input.ident;
		let packet_id = get_packet_id(&input.attrs);

    let serialization_fields = input.fields.iter().map(|field| {
        let field_name = &field.ident.clone().unwrap();

        match field.ty {
            Type::Array(syn::TypeArray { ref len, ref elem, .. }) => {
				match elem.as_ref() {
					Type::Path(syn::TypePath {ref path, .. }) => {
						let type_name = &*path.segments.last().unwrap().ident.to_string();
						let method_name = format_ident!("write_{}", type_name);

						match type_name {
							"u8" => quote! { buf.write_all(&self.#field_name).ok()?; },
							"i8" => quote! {
								let mut i = 0usize;
								while i < #len {
									buf.write_i8(self.#field_name[i]).ok()?;
									i += 1;
								}
							},
							_ => quote! {
								let mut i = 0usize;
								while i < #len {
									buf.#method_name::<LittleEndian>()(self.#field_name[i]).ok()?;
									i += 1;
								}
							}
						}
					},
					_ => quote! {}
				}
			}
			Type::Path(syn::TypePath { ref path, ..}) => match path.segments.last().unwrap().ident.to_string().as_str() {
				"u8" => quote! { buf.write_all(&[self.#field_name]).ok()?; },
				"Vec" => match &path.segments.first().unwrap().arguments {
					PathArguments::AngleBracketed(gargs) => {
						match gargs.args.first().unwrap() {
							GenericArgument::Type(ty) =>  match ty {
								Type::Path(syn::TypePath { ref path, ..}) => match path.segments.last().unwrap().ident.to_string() {
									x if x.starts_with("u") || x.starts_with("i") => {
										let serialize_method_name = format_ident!("write_{}", x);
										if x == "u8" || x == "i8" {
											quote! {
												for item in self.#field_name.iter() {
													buf.#serialize_method_name(*item).ok()?
												}
											}
										} else {
											quote! {
												for item in self.#field_name.iter() {
													buf.#serialize_method_name::<LittleEndian>(*item).ok()?
												}
											}
										}
									},
									_ => quote! {
										for fragment in self.#field_name.iter() {
											if let Some(mut serialized) = fragment.serialize() {
												buf.append(&mut serialized);
											}
										}
									}
								},
								_ => quote! {},
							},
							_ => quote! {}
						}
					},
					_ => quote! {}
				},
				_ => quote! { buf.write_all(&self.#field_name.to_le_bytes()).ok()?; }
			},
            _ => quote! {}
        }
    });

	let deserialization_fields = input.fields.iter().map(|field| {
        let field_name = &field.ident.clone().unwrap();

        match field.ty {
            Type::Array(syn::TypeArray { ref len, ref elem, .. }) => {
				match elem.as_ref() {
					Type::Path(syn::TypePath {ref path, .. }) => {
						let type_name = &*path.segments.last().unwrap().ident.to_string();
						let method_name = format_ident!("read_{}", type_name);

						match type_name {
							"u8" => quote! {
								cursor.read_exact(&mut packet.#field_name).ok()?;
							},
							"i8" => quote! {
								let mut limited_cursor = cursor.take(#len as u64);
								let mut i = 0usize;
								while i < #len {
									packet.#field_name[i] = limited_cursor.read_i8().ok()?;
									i += 1;
								}
							},
							_ => quote! {
								let mut limited_cursor = cursor.take(#len as u64);
								let i = 0usize;
								while i < #len {
									packet.#field_name[i] = limited_cursor.#method_name::<LittleEndian>().ok()?;
									i += 1;
								}
							}
						}
					},
					_ => quote! {}
				}
			}, // for [T; n]
			Type::Path(syn::TypePath { ref path, ..}) => {
				let type_name = &*path.segments.last().unwrap().ident.to_string();
				let method_name = format_ident!("read_{}", type_name);
				match type_name {
					"u8" => quote! { packet.#field_name = cursor.read_u8().ok()?; },
					"Vec" => match &path.segments.first().unwrap().arguments {
						PathArguments::AngleBracketed(gargs) => {
							match gargs.args.first().unwrap() {
								GenericArgument::Type(ty) =>  match ty {
									Type::Path(syn::TypePath { ref path, ..}) => match path.segments.last().unwrap().ident.to_string() {
										x if x.starts_with("u") || x.starts_with("i") => {
											let deserialize_method_name = format_ident!("read_{}", x);
											if x == "u8" || x == "i8" {
												quote! {
													while (cursor.position() as usize) < length {
														packet.#field_name.push(cursor.#deserialize_method_name().ok()?);
													}
												}
											} else {
												quote! {
													while (cursor.position() as usize) < length {
														packet.#field_name.push(cursor.#deserialize_method_name::<LittleEndian>().ok()?);
													}
												}
											}
										},
										_ => quote! {
											while (cursor.position() as usize) < length {
												if let Some(item) = <#ty>::deserialize(cursor) {
													packet.#field_name.push(item);
												} else {
													break;
												}
											}
										}
									},
									_ => quote! {},
								},
								_ => quote! {}
							}
						},
						_ => quote! {}
					},
					_ => quote! { packet.#field_name = cursor.#method_name::<LittleEndian>().ok()?; }
				}
			},
            _ => quote! {}
        }
    });

	let length_fields = input.fields.iter().map(|field| {
		let field_type = &field.ty;

        match field.ty {
            Type::Array(syn::TypeArray { ref len, .. }) => quote! { fixed_len += #len; }, // for [T; n]
			Type::Path(syn::TypePath { ref path, .. }) => match path.segments.last().unwrap().ident.to_string().as_str() {
				"Vec" => match &path.segments.first().unwrap().arguments {
					PathArguments::AngleBracketed(gargs) => {
						match gargs.args.first().unwrap() {
							GenericArgument::Type(ty) =>  match ty {
								Type::Path(syn::TypePath { ref path, ..}) => match path.segments.last().unwrap().ident.to_string() {
									x if x.starts_with("u") || x.starts_with("i") => {
										quote! { variable_len += std::mem::size_of::<#ty>(); }
									},
									_ => quote! { variable_len += #ty::get_base_len(); }
								},
								_ => quote! {},
							},
							_ => quote! {}
						}
					},
					_ => quote! {}
				},
				_ => quote! { fixed_len += std::mem::size_of::<#field_type>(); }
			},
            _ => quote! {}
        }
    });

    // Generate the implementation of the Packet trait
    let expanded = quote! {
		impl Packet for #struct_name {
			fn new() -> Self {
				let mut packet = Self::default();

				packet.packet_id = #packet_id as u16;
				packet
			}

			fn serialize(&self) -> Option<Vec<u8>> {
				use std::io::Write;
				use byteorder::{LittleEndian, WriteBytesExt};
				let mut buf: Vec<u8> = vec![];

				#(#serialization_fields)*

				Some(buf)
			}

			fn deserialize(buffer: &[u8]) -> Option<Self>
			where
				Self: Default,
			{
				use std::io::{Read, Cursor};
				use byteorder::{LittleEndian, ReadBytesExt};

				let mut cursor = &mut Cursor::new(buffer);
				let mut packet = Self::default();
				let length = cursor.get_ref().len() as usize;

				if !packet.has_valid_length(length) {
					return None;
				}

				#(#deserialization_fields)*

				Some(packet)
			}

			fn len(&self) -> usize {
				// I don't think this is the best way to get the length
				if let Some(buf) = self.serialize() {
					return buf.len();
				}
				0usize
			}

			fn has_valid_length(&self, length: usize) -> bool {
				let mut fixed_len = 0usize;
				let mut variable_len = 0usize;
				
				#(#length_fields)*

				if length - fixed_len == 0 || (variable_len > 0 && (length - fixed_len) % variable_len == 0) {
					true
				} else {
					false
				}
			}
		}
    };

    TokenStream::from(expanded)
}



#[proc_macro_derive(PacketFragment)]
pub fn packet_fragment_derive(input: TokenStream) -> TokenStream {
    let input: ItemStruct = parse_macro_input!(input as ItemStruct);

    let struct_name = &input.ident;

    let serialization_fields = input.fields.iter().map(|field| {
        let field_name = &field.ident.clone().unwrap();

        match field.ty {
            Type::Array(_) => { quote! { buf.write_all(&self.#field_name).ok()?; } } // for [T; n]
			Type::Path(syn::TypePath { ref path, ..}) => match path.segments.last().unwrap().ident.to_string().as_str() {
				"u8" => quote! { buf.write_all(&[self.#field_name]).ok()?; },
				"Vec" => quote! { }, // Fragments can't have nested fields or array fields.
				_ => quote! { buf.write_all(&self.#field_name.to_le_bytes()).ok()?; }
			},
            _ => { quote! {} }
        }
    });

	let deserialization_fields = input.fields.iter().map(|field| {
        let field_name = &field.ident.clone().unwrap();

        match field.ty {
            Type::Array(_) => quote! { cursor.read_exact(&mut fragment.#field_name).ok()?; }, // for [T; n]
			Type::Path(syn::TypePath { ref path, ..}) => {
				let type_name = &*path.segments.last().unwrap().ident.to_string();
				let method_name = format_ident!("read_{}", type_name);
				match type_name {
					"u8" => quote! { fragment.#field_name = cursor.read_u8().ok()?; },
					"Vec" => quote! {},
					_ => quote! { fragment.#field_name = cursor.#method_name::<LittleEndian>().ok()?; }
				}
			},
            _ => quote! {}
        }
    });

	let fixed_length_fields = input.fields.iter().map(|field| {
		let field_type = &field.ty;

        match field.ty {
            Type::Array(syn::TypeArray { ref len, .. }) => quote! { base_len += #len; }, // for [T; n]
			Type::Path(syn::TypePath { ref path, .. }) => match path.segments.last().unwrap().ident.to_string().as_str() {
				"Vec" => quote! {},
				_ => quote! { base_len += std::mem::size_of::<#field_type>(); }
			},
            _ => quote! {}
        }
    });

    // Generate the implementation of the Packet trait
    let expanded = quote! {
		impl PacketFragment for #struct_name {
			fn serialize(&self) -> Option<Vec<u8>> {
				use std::io::Write;
				use byteorder::{LittleEndian, WriteBytesExt};
				let mut buf: Vec<u8> = vec![];

				#(#serialization_fields)*

				Some(buf)
			}

			fn get_base_len() -> usize {
				let mut base_len = 0usize; 
				
				#(#fixed_length_fields)*
				
				base_len
			}

			fn deserialize(cursor: &mut std::io::Cursor<&[u8]>) -> Option<Self> {
				use byteorder::{LittleEndian, ReadBytesExt};
				use std::io::Read;
				
				let mut fragment = Self::default();
				let length = cursor.get_ref().len();

				if (length - cursor.position() as usize) < Self::get_base_len() {
					return None;
				}

				#(#deserialization_fields)*

				Some(fragment)
			}
		}
    };

    TokenStream::from(expanded)
}
